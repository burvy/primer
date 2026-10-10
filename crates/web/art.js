// Small helpers for hand-drawn art. Called from Rust (src/art.rs).
//
//   primerArt.writeIn(container, boxId, text, x, y)
//
// Writes `text` centred in the element with id `boxId` inside `container`'s
// drawing: the function machine's `label-box` ("+1") and the cover's
// `name-box` (the hero's name). The artist draws an empty box and never types
// text; this puts the words in. If the drawing has no such box, the text goes
// at (x, y) of the drawing instead, so an older drawing still shows it.
//
// Placing text needs the box's size on screen, which only the browser knows
// (`getBBox`), so this is plain JavaScript rather than Rust.

(function () {
  "use strict";

  function writeIn(container, boxId, text, x, y) {
    const svg = container.querySelector("svg");
    if (!svg) return;
    svg.querySelectorAll("text.art-text").forEach((old) => old.remove());

    const label = document.createElementNS("http://www.w3.org/2000/svg", "text");
    label.setAttribute("class", "art-text");
    label.setAttribute("text-anchor", "middle");
    label.setAttribute("dominant-baseline", "central");
    label.textContent = text;

    const box = svg.querySelector("#" + boxId);
    if (box) {
      const b = box.getBBox();
      // As tall as the box allows, but never wider than it.
      const size = Math.min(b.height * 0.75, (b.width / Math.max(text.length, 2)) * 1.6);
      label.setAttribute("x", b.x + b.width / 2);
      label.setAttribute("y", b.y + b.height / 2);
      label.setAttribute("font-size", size);
      // Same parent as the box, so the box's own transforms apply to the text.
      box.parentNode.appendChild(label);
    } else {
      label.setAttribute("x", x);
      label.setAttribute("y", y);
      label.setAttribute("font-size", 28);
      svg.appendChild(label);
    }
  }

  window.primerArt = { writeIn };
})();
