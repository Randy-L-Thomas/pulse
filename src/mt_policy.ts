/** Frozen Translate contract: lexicon by default. Ollama only on the LLM button. */
export const AUTO_LLM_AFTER_LEX = false;

/** Later LLM click must survive a lex retry. Lex cannot downgrade a queued LLM. */
export function mergeQueuedLlm(queuedLlm: boolean, incomingLlm: boolean): boolean {
  return queuedLlm || incomingLlm;
}
