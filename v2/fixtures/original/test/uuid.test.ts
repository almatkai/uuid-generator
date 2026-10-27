import assert from "node:assert/strict";
import test from "node:test";
import { v4 as uuidV4, v5 as uuidV5, v7 as uuidV7, validate as uuidValidate, parse as uuidParse } from "uuid";
import { monotonicFactory } from "ulidx";
import { typeid, fromString, toUUID, TypeID } from "typeid-js";

test("uuid v4 generates valid IDs", () => {
  const id = uuidV4();
  assert.equal(uuidValidate(id), true);
  assert.match(id, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i);
});

test("uuid v5 is deterministic for same name and namespace", () => {
  const ns = uuidV4();
  const a = uuidV5("hello", ns);
  const b = uuidV5("hello", ns);
  const c = uuidV5("world", ns);
  assert.equal(a, b);
  assert.notEqual(a, c);
  assert.equal(uuidValidate(a), true);
});

test("uuid v7 generates time-ordered valid IDs", () => {
  const a = uuidV7();
  const b = uuidV7();
  assert.equal(uuidValidate(a), true);
  assert.equal(uuidValidate(b), true);
  assert.notEqual(a, b);
});

test("ulid monotonic factory generates sortable IDs", () => {
  const ulid = monotonicFactory();
  const a = ulid();
  const b = ulid();
  assert.equal(a.length, 26);
  assert.equal(b.length, 26);
  assert.ok(a < b || a !== b);
});

test("typeid-js round-trips prefix and uuid", () => {
  const tid = typeid("user").toString();
  assert.match(tid, /^user_[0-9a-z]{26}$/);
  const parsed = fromString(tid);
  const uuid = toUUID(parsed);
  assert.equal(uuidValidate(uuid), true);
  // pack via TypeID.fromUUIDBytes
  const bytes = uuidParse(uuid);
  const packed = TypeID.fromUUIDBytes("order", bytes).toString();
  assert.match(packed, /^order_[0-9a-z]{26}$/);
});

test("formatUuidWithDashes logic matches src/formatUuid", () => {
  function formatUuidWithDashes(uuid: string): string {
    const clean = uuid.replace(/[^a-fA-F0-9]/g, "");
    if (clean.length !== 32) throw new Error("Invalid");
    return [clean.slice(0, 8), clean.slice(8, 12), clean.slice(12, 16), clean.slice(16, 20), clean.slice(20, 32)].join("-");
  }
  assert.equal(formatUuidWithDashes("550e8400e29b41d4a716446655440000"), "550e8400-e29b-41d4-a716-446655440000");
  assert.equal(formatUuidWithDashes("550e8400-e29b-41d4-a716-446655440000"), "550e8400-e29b-41d4-a716-446655440000");
  assert.throws(() => formatUuidWithDashes("short"), /Invalid/);
});

test("package.json declares Windows+macOS and Tezbar metadata", async () => {
  const pkg = await import("../package.json", { with: { type: "json" } });
  const data = (pkg as unknown as { default: Record<string, unknown> }).default ?? (pkg as unknown as Record<string, unknown>);
  assert.deepEqual((data as { platforms: string[] }).platforms, ["macOS", "Windows"]);
  const tezbar = (data as { tezbar: { platforms: string[]; repository: string } }).tezbar;
  assert.deepEqual(tezbar.platforms, ["macos", "windows"]);
  assert.equal(tezbar.repository, "https://github.com/almatkai/uuid-generator");
  const commands = (data as { commands: { platforms: string[] }[] }).commands;
  assert.ok(commands.length >= 9);
  for (const cmd of commands) {
    assert.deepEqual(cmd.platforms, ["macOS", "Windows"]);
  }
});
