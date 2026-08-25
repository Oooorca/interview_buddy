export function looksLikeInterviewPrompt(text: string): boolean {
  const compact = text.replace(/\s+/g, " ").trim();
  if (compact.length < 4) return false;
  if (/[?？]$/.test(compact)) return true;
  return /(请问|怎么|如何|为什么|什么|哪些|是否|能否|可以|讲讲|介绍一下|说说|解释|区别|优缺点|复杂度|实现|设计)/.test(compact)
    || /\b(what|why|how|when|where|who|which|can|could|would|do|does|did|is|are|tell me|explain|describe|compare|implement|design|complexity)\b/i.test(compact)
    || /(どのよう|なぜ|何|説明|教えて|어떻게|왜|무엇|설명|comment|pourquoi|quoi|explique|cómo|por qué|qué|explica|warum|was|wie|erkläre)/i.test(compact);
}

export function normalizedQuestion(text: string): string {
  return text.toLocaleLowerCase().replace(/[\s?？。,.!！:：;；'"“”‘’]/g, "");
}
