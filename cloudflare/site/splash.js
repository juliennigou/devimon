/**
 * Splash screen — reveals DEVIMON letter-by-letter in ASCII block art,
 * then shows the slogan, then fades out to reveal the main page.
 * Total duration: ~3.5 seconds.
 */
(function () {
  const LETTERS = {
    D: [
      "██████╗ ",
      "██╔══██╗",
      "██║  ██║",
      "██║  ██║",
      "██████╔╝",
      "╚═════╝ ",
    ],
    E: [
      "███████╗",
      "██╔════╝",
      "█████╗  ",
      "██╔══╝  ",
      "███████╗",
      "╚══════╝",
    ],
    V: [
      "██╗   ██╗",
      "██║   ██║",
      "██║   ██║",
      "╚██╗ ██╔╝",
      " ╚████╔╝ ",
      "  ╚═══╝  ",
    ],
    I: [
      "██╗",
      "██║",
      "██║",
      "██║",
      "██║",
      "╚═╝",
    ],
    M: [
      "███╗   ███╗",
      "████╗ ████║",
      "██╔████╔██║",
      "██║╚██╔╝██║",
      "██║ ╚═╝ ██║",
      "╚═╝     ╚═╝",
    ],
    O: [
      " ██████╗ ",
      "██╔═══██╗",
      "██║   ██║",
      "██║   ██║",
      "╚██████╔╝",
      " ╚═════╝ ",
    ],
    N: [
      "███╗   ██╗",
      "████╗  ██║",
      "██╔██╗ ██║",
      "██║╚██╗██║",
      "██║ ╚████║",
      "╚═╝  ╚═══╝",
    ],
  };

  const WORD = ["D", "E", "V", "I", "M", "O", "N"];
  const SLOGAN = "Raise terminal monsters. Climb ranks.";
  const LETTER_DELAY = 200;    // ms between each letter appearing
  const SLOGAN_DELAY = 400;    // ms after last letter before slogan
  const HOLD_DURATION = 800;   // ms to hold the full splash
  const FADE_DURATION = 500;   // ms for fade-out transition

  const splashEl = document.getElementById("splash");
  const artEl = document.getElementById("splash-art");
  const sloganEl = document.getElementById("splash-slogan");
  const pageEl = document.getElementById("page");

  if (!splashEl || !artEl || !pageEl) return;

  // Build 6 empty rows
  let rows = ["", "", "", "", "", ""];
  let letterIndex = 0;

  function addNextLetter() {
    if (letterIndex >= WORD.length) {
      // All letters shown — show slogan then fade
      setTimeout(showSlogan, SLOGAN_DELAY);
      return;
    }

    const letter = LETTERS[WORD[letterIndex]];
    for (let r = 0; r < 6; r++) {
      rows[r] += (letterIndex > 0 ? " " : "") + letter[r];
    }
    artEl.textContent = rows.join("\n");
    letterIndex++;
    setTimeout(addNextLetter, LETTER_DELAY);
  }

  function showSlogan() {
    sloganEl.textContent = SLOGAN;
    sloganEl.classList.add("visible");
    setTimeout(fadeOut, HOLD_DURATION);
  }

  function fadeOut() {
    splashEl.classList.add("fade-out");
    setTimeout(() => {
      splashEl.style.display = "none";
      pageEl.classList.remove("hidden");
    }, FADE_DURATION);
  }

  // Kick off
  setTimeout(addNextLetter, 300);
})();
