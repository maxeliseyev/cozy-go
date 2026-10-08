// The same synchronous WASM bridge runs in browsers and Node.js.
import { WASM_BASE64 } from "./wasm-data";

type Exports = {
  memory: WebAssembly.Memory;
  cozy_web_version(): number;
  cozy_web_input_ptr(): number;
  cozy_web_input_capacity(): number;
  cozy_web_output_ptr(): number;
  cozy_web_run(operation: number, length: number): number;
};

export class RulesError extends Error {
  constructor(public readonly code: string) {
    super(code);
    this.name = "RulesError";
  }
}

let wasm: Exports | undefined;
function engine(): Exports {
  if (!wasm) {
    const bytes = Uint8Array.from(atob(WASM_BASE64), (byte) =>
      byte.charCodeAt(0),
    );
    const module = new WebAssembly.Module(bytes);
    if (WebAssembly.Module.imports(module).length !== 0)
      throw new RulesError("unexpected_imports");
    wasm = new WebAssembly.Instance(module).exports as unknown as Exports;
    if (wasm.cozy_web_version() !== 1)
      throw new RulesError("unsupported_engine");
  }
  return wasm;
}

function request(
  size: number,
  board: readonly number[],
  extra: number,
): Uint8Array {
  if (![9, 13, 19].includes(size) || board.length !== size * size)
    throw new RulesError("invalid_input");
  if (board.some((cell) => !Number.isInteger(cell) || cell < 0 || cell > 2))
    throw new RulesError("invalid_input");
  const bytes = new Uint8Array(1 + board.length + extra);
  bytes[0] = size;
  bytes.set(board, 1);
  return bytes;
}

function run(operation: number, bytes: Uint8Array): Uint8Array {
  const exports = engine();
  if (bytes.length > exports.cozy_web_input_capacity())
    throw new RulesError("invalid_input");
  new Uint8Array(
    exports.memory.buffer,
    exports.cozy_web_input_ptr(),
    bytes.length,
  ).set(bytes);
  const length = exports.cozy_web_run(operation, bytes.length);
  if (length === 0) throw new RulesError("invalid_input");
  const output = new Uint8Array(
    exports.memory.buffer,
    exports.cozy_web_output_ptr(),
    length,
  ).slice();
  const errors = [
    "",
    "occupied",
    "suicide",
    "superko",
    "outside_board",
    "game_finished",
    "invalid_input",
  ];
  if (output[0] !== 0)
    throw new RulesError(errors[output[0]] ?? "invalid_output");
  return output;
}

function integer(value: number, max: number): void {
  if (!Number.isInteger(value) || value < 0 || value > max)
    throw new RulesError("invalid_input");
}

/** Stones only; callers use this for positional-superko history. */
export function positionHash(size: number, board: readonly number[]): bigint {
  const output = run(1, request(size, board, 0));
  return new DataView(output.buffer).getBigUint64(1, true);
}

/** Group membership and distinct liberties. Indices start at the lower-left corner. */
export function group(
  size: number,
  board: readonly number[],
  point: number,
): { stones: number[]; liberties: number[] } {
  integer(point, 65535);
  const bytes = request(size, board, 2);
  new DataView(bytes.buffer).setUint16(1 + board.length, point, true);
  const output = run(2, bytes);
  const view = new DataView(output.buffer);
  const stones = view.getUint16(1, true),
    liberties = view.getUint16(3, true);
  return {
    stones: Array.from({ length: stones }, (_, index) =>
      view.getUint16(5 + index * 2, true),
    ),
    liberties: Array.from({ length: liberties }, (_, index) =>
      view.getUint16(5 + (stones + index) * 2, true),
    ),
  };
}

/** Chinese area, territory ownership, and komi, entirely calculated by cozy-go. */
export function areaScore(
  size: number,
  board: readonly number[],
  dead: readonly number[],
  komiHalfPoints: number,
): { black: number; whiteHalfPoints: number; territory: number[] } {
  integer(komiHalfPoints, 2147483647);
  integer(dead.length, 361);
  dead.forEach((point) => integer(point, 65535));
  const bytes = request(size, board, 6 + dead.length * 2);
  const view = new DataView(bytes.buffer),
    offset = 1 + board.length;
  view.setInt32(offset, komiHalfPoints, true);
  view.setUint16(offset + 4, dead.length, true);
  dead.forEach((point, index) =>
    view.setUint16(offset + 6 + index * 2, point, true),
  );
  const output = run(3, bytes),
    result = new DataView(output.buffer);
  return {
    black: result.getUint32(1, true),
    whiteHalfPoints: result.getUint32(5, true),
    territory: Array.from(output.subarray(9)),
  };
}

/** Restores and applies one move with capture, suicide, superko, turn and pass rules. */
export function play(
  size: number,
  board: readonly number[],
  turn: number,
  passes: number,
  point: number | null,
  komiHalfPoints: number,
  history: readonly bigint[],
): {
  board: number[];
  turn: number;
  passes: number;
  captures: number;
  hash: bigint;
} {
  if (turn !== 1 && turn !== 2) throw new RulesError("invalid_input");
  integer(passes, 2);
  if (point !== null) integer(point, 65534);
  integer(komiHalfPoints, 2147483647);
  integer(history.length, 10001);
  if (
    history.some(
      (hash) =>
        typeof hash !== "bigint" || hash < 0n || hash > 0xffffffffffffffffn,
    )
  )
    throw new RulesError("invalid_input");
  const bytes = request(size, board, 12 + history.length * 8);
  const view = new DataView(bytes.buffer),
    offset = 1 + board.length;
  view.setUint8(offset, turn);
  view.setUint8(offset + 1, passes);
  view.setUint16(offset + 2, point === null ? 65535 : point, true);
  view.setInt32(offset + 4, komiHalfPoints, true);
  view.setUint32(offset + 8, history.length, true);
  history.forEach((hash, index) =>
    view.setBigUint64(offset + 12 + index * 8, hash, true),
  );
  const output = run(4, bytes),
    result = new DataView(output.buffer);
  return {
    board: Array.from(output.subarray(13)),
    turn: output[1],
    passes: output[2],
    captures: result.getUint16(3, true),
    hash: result.getBigUint64(5, true),
  };
}
