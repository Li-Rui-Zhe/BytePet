const TOOL_CALL_START = "<tool_call";
const TOOL_CALL_END = "</tool_call>";
const ACTION_TAG_START = "[[pet_action:";
const HIDDEN_MARKERS = [TOOL_CALL_START, ACTION_TAG_START];

export interface StreamBufferPartition {
  visible: string;
  pending: string;
}

function markerPrefixAtEnd(text: string): number {
  const lower = text.toLowerCase();
  let longest = 0;

  for (const marker of HIDDEN_MARKERS) {
    const maxLength = Math.min(lower.length, marker.length - 1);
    for (let length = maxLength; length > longest; length -= 1) {
      if (lower.endsWith(marker.slice(0, length))) {
        longest = length;
        break;
      }
    }
  }

  return longest;
}

export function partitionStreamBuffer(text: string): StreamBufferPartition {
  let remaining = text;
  let visible = "";

  while (remaining) {
    const lower = remaining.toLowerCase();
    const toolStart = lower.indexOf(TOOL_CALL_START);
    const actionStart = lower.indexOf(ACTION_TAG_START);
    const hiddenStarts = [toolStart, actionStart].filter((index) => index >= 0);

    if (hiddenStarts.length === 0) {
      const pendingLength = markerPrefixAtEnd(remaining);
      const visibleLength = remaining.length - pendingLength;
      return {
        visible: visible + remaining.slice(0, visibleLength),
        pending: remaining.slice(visibleLength),
      };
    }

    const hiddenStart = Math.min(...hiddenStarts);
    visible += remaining.slice(0, hiddenStart);

    if (hiddenStart === actionStart) {
      return { visible, pending: remaining.slice(hiddenStart) };
    }

    const toolEnd = lower.indexOf(TOOL_CALL_END, hiddenStart);
    if (toolEnd < 0) {
      return { visible, pending: remaining.slice(hiddenStart) };
    }

    remaining = remaining.slice(toolEnd + TOOL_CALL_END.length);
  }

  return { visible, pending: "" };
}
