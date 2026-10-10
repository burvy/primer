// Small helpers for hand-drawn art. Called from Rust (src/art.rs).
//
//   primerArt.prepare(container)   tidy a just-loaded machine drawing
//   primerArt.writeIn(container, boxId, text, x, y)   put words in a box
//   primerArt.stairs(holder)       lay joined machines out as a staircase
//
// Everything here needs sizes and positions on screen, which only the browser
// knows (`getBBox`), so it is plain JavaScript rather than Rust. The artist
// draws shapes and names the layers; nothing here depends on where they are.

(function () {
  "use strict";

  const SVG = "http://www.w3.org/2000/svg";

  // The layers `play.css` shows, hides or moves by id.
  const LAYERS = [
    "body", "funnel", "chute", "front-panel", "pipe", "input-glow",
    "output-glow", "hatch-closed", "hatch-open", "block", "gear",
  ];

  // The box of `el` in the drawing's own coordinates (the viewBox), whatever
  // transforms sit on the element or its layers.
  function box(svg, el) {
    const b = el.getBBox();
    const m = svg.getScreenCTM().inverse().multiply(el.getScreenCTM());
    const xs = [];
    const ys = [];
    for (const [x, y] of [
      [b.x, b.y], [b.x + b.width, b.y], [b.x, b.y + b.height], [b.x + b.width, b.y + b.height],
    ]) {
      const p = new DOMPoint(x, y).matrixTransform(m);
      xs.push(p.x);
      ys.push(p.y);
    }
    const left = Math.min(...xs);
    const top = Math.min(...ys);
    return { x: left, y: top, width: Math.max(...xs) - left, height: Math.max(...ys) - top };
  }

  // Inkscape writes `style="display:none"` (and opacity) on a layer that was
  // hidden or faded while drawing. Those inline styles would beat the
  // stylesheet, so the known layers lose them and the CSS decides.
  function clean(svg) {
    for (const id of LAYERS) {
      const el = svg.querySelector("#" + id);
      if (!el) continue;
      for (const name of ["display", "visibility", "opacity"]) {
        el.style.removeProperty(name);
        el.removeAttribute(name);
      }
    }
  }

  // The block moves from the funnel's mouth to the chute's exit. Its layer
  // already carries a transform from the drawing, so it is wrapped in a group
  // that CSS can move; the two ends are set as custom properties on it.
  function wireBlock(svg) {
    const block = svg.querySelector("#block");
    if (!block || block.parentNode.classList.contains("block-mover")) return;
    const mover = document.createElementNS(SVG, "g");
    mover.setAttribute("class", "block-mover");
    block.parentNode.insertBefore(mover, block);
    mover.appendChild(block);

    const funnel = svg.querySelector("#funnel");
    const chute = svg.querySelector("#chute");
    if (!funnel || !chute) return;
    const at = box(svg, block);
    const from = box(svg, funnel);
    const to = box(svg, chute);
    const cx = at.x + at.width / 2;
    const cy = at.y + at.height / 2;
    // Start with the block's middle at the middle of the funnel's mouth, end
    // with it at the middle of the chute's bottom edge.
    mover.style.setProperty("--bx0", from.x + from.width / 2 - cx + "px");
    mover.style.setProperty("--by0", from.y + at.height / 2 - cy + "px");
    mover.style.setProperty("--bx1", to.x + to.width / 2 - cx + "px");
    mover.style.setProperty("--by1", to.y + to.height - at.height / 2 - cy + "px");
  }

  // The front panel is the plate the label goes on. It is lifted above the
  // hatches, so a closed hatch (which covers the whole body) does not hide it.
  function liftPanel(svg) {
    const panel = svg.querySelector("#front-panel");
    const hatch = svg.querySelector("#hatch-open") || svg.querySelector("#hatch-closed");
    if (panel && hatch && hatch.parentNode === panel.parentNode) {
      hatch.after(panel);
    }
  }

  function prepare(container) {
    const svg = container.querySelector("svg");
    if (!svg) return;
    clean(svg);
    liftPanel(svg);
    wireBlock(svg);
  }

  // Writes `text` centred in the element with id `boxId`: the machine's
  // `label-box`, the cover's `name-box`. A machine without a `label-box` gets
  // the text in its `front-panel` instead. With neither, the text goes at
  // (x, y).
  function writeIn(container, boxId, text, x, y) {
    const svg = container.querySelector("svg");
    if (!svg) return;
    svg.querySelectorAll("text.art-text").forEach((old) => old.remove());

    const label = document.createElementNS(SVG, "text");
    label.setAttribute("class", "art-text");
    label.setAttribute("text-anchor", "middle");
    label.setAttribute("dominant-baseline", "central");
    label.textContent = text;

    const target = svg.querySelector("#" + boxId) || svg.querySelector("#front-panel");
    if (target) {
      const b = target.getBBox();
      // As tall as the box allows, but never wider than it.
      const size = Math.min(b.height * 0.75, (b.width / Math.max(text.length, 2)) * 1.6);
      label.setAttribute("x", b.x + b.width / 2);
      label.setAttribute("y", b.y + b.height / 2);
      label.setAttribute("font-size", size);
      // Inside a group, so the group's own transform applies to the text;
      // beside a plain shape, so the shape's parent's transform does.
      (target.tagName === "g" ? target : target.parentNode).appendChild(label);
    } else {
      label.setAttribute("x", x);
      label.setAttribute("y", y);
      label.setAttribute("font-size", 28);
      svg.appendChild(label);
    }
  }

  // Lays the `.machine` children of `holder` out as a staircase: each one's
  // funnel mouth (top middle of #funnel) sits right under the one before's
  // chute exit (bottom middle of #chute). The step comes from the drawing, so
  // a redraw cannot break it. Sizes are percentages of the holder, so the
  // whole staircase scales to the column.
  function stairs(holder) {
    const machines = Array.from(holder.querySelectorAll(":scope > .machine"));
    const svg = machines.length && machines[0].querySelector("svg");
    if (!svg) return;
    machines.forEach((m) => prepare(m));

    const vb = svg.viewBox.baseVal;
    const funnel = svg.querySelector("#funnel");
    const chute = svg.querySelector("#chute");
    let dx = vb.width * 0.45;
    let dy = vb.height * 0.7;
    if (funnel && chute) {
      const f = box(svg, funnel);
      const c = box(svg, chute);
      dx = c.x + c.width / 2 - (f.x + f.width / 2);
      dy = c.y + c.height - f.y;
    }
    const last = machines.length - 1;
    const minX = Math.min(0, last * dx);
    const minY = Math.min(0, last * dy);
    const width = vb.width + Math.abs(last * dx);
    const height = vb.height + Math.abs(last * dy);

    holder.style.aspectRatio = width + " / " + height;
    machines.forEach((m, k) => {
      m.style.left = ((k * dx - minX) / width) * 100 + "%";
      m.style.top = ((k * dy - minY) / height) * 100 + "%";
      m.style.width = (vb.width / width) * 100 + "%";
      m.style.setProperty("--step", k); // the block and gear go one after another
    });
  }

  window.primerArt = { prepare, writeIn, stairs };
})();
