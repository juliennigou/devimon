/**
 * Reusable ASCII UI helpers for the Devimon site.
 */

/** Stage-specific ASCII monster for table rows. */
function stageAscii(stage) {
  switch (stage?.toLowerCase()) {
    case "baby":
      return "( o_o )";
    case "young":
      return "\\( o_o )/";
    case "evolved":
      return "\\\\( O_O )//";
    default:
      return "( ?.? )";
  }
}

/** CSS class for the stage pill. */
function stageClass(stage) {
  switch (stage?.toLowerCase()) {
    case "baby":
      return "stage-baby";
    case "young":
      return "stage-young";
    case "evolved":
      return "stage-evolved";
    default:
      return "";
  }
}

/** Rank decoration for top 3. */
function rankDisplay(rank) {
  if (rank == null) {
    return { text: "—", cls: "rank-unverified" };
  }
  switch (rank) {
    case 1:
      return { text: "♛ 1", cls: "rank-1" };
    case 2:
      return { text: "♕ 2", cls: "rank-2" };
    case 3:
      return { text: "♗ 3", cls: "rank-3" };
    default:
      return { text: `${rank}`, cls: "" };
  }
}

function verificationDisplay(status) {
  switch ((status || "").toLowerCase()) {
    case "verified":
      return { text: "Verified", cls: "trust-verified" };
    default:
      return { text: "Unverified", cls: "trust-unverified" };
  }
}

/** Human-readable relative time. */
function timeAgo(value) {
  const timestamp = new Date(value);
  if (Number.isNaN(timestamp.getTime())) return "Unknown";

  const diffSeconds = Math.max(
    0,
    Math.floor((Date.now() - timestamp.getTime()) / 1000)
  );
  if (diffSeconds < 60) return `${diffSeconds}s ago`;
  if (diffSeconds < 3600) return `${Math.floor(diffSeconds / 60)}m ago`;
  if (diffSeconds < 86400) return `${Math.floor(diffSeconds / 3600)}h ago`;
  return `${Math.floor(diffSeconds / 86400)}d ago`;
}

/** Sanitize HTML to prevent injection. */
function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}
