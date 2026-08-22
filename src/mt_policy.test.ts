import assert from "node:assert/strict";
import { test } from "node:test";
import { AUTO_LLM_AFTER_LEX, mergeQueuedLlm } from "./mt_policy.ts";

test("lex path never auto-starts Ollama", () => {
  assert.equal(AUTO_LLM_AFTER_LEX, false);
});

test("queued LLM click is not downgraded to lex", () => {
  assert.equal(mergeQueuedLlm(false, true), true);
  assert.equal(mergeQueuedLlm(true, false), true);
});

test("queued lex stays lex until LLM is requested", () => {
  assert.equal(mergeQueuedLlm(false, false), false);
});
