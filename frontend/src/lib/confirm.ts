const DEFAULT_SECOND =
  "请再次确认。此操作无法撤销，确定继续吗？";

/** 破坏性操作需连续确认两次。 */
export function confirmTwice(
  firstMessage: string,
  secondMessage: string = DEFAULT_SECOND,
): boolean {
  if (!window.confirm(firstMessage)) return false;
  return window.confirm(secondMessage);
}
