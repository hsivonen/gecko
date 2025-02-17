/* vim: set ft=javascript ts=2 et sw=2 tw=80: */
/* Any copyright is dedicated to the Public Domain.
  http://creativecommons.org/publicdomain/zero/1.0/ */

"use strict";

// Tests that the rule view's highlightProperty scrolls to the specified declaration.

const TEST_URI = `
  <style type="text/css">
    .test {
      font-size: 12px;
      border: 1px solid blue;
    }

    .test::after {
      content: "!";
      color: red;
    }
  </style>
  <div class="test">Hello this is a test</div>
`;

add_task(async function() {
  await addTab("data:text/html;charset=utf-8," + encodeURIComponent(TEST_URI));
  const { inspector, view } = await openRuleView();

  await selectNode(".test", inspector);
  const { rules, styleWindow } = view;

  info("Highlight the computed border-left-width declaration in the rule view.");
  const borderLeftWidthStyle = rules[2].textProps[1].computed
    .find(({ name }) => name === "border-left-width");

  let onHighlightProperty = view.once("scrolled-to-element");
  let isHighlighted = view.highlightProperty("border-left-width");
  await onHighlightProperty;

  ok(isHighlighted, "border-left-property is highlighted.");
  ok(isInViewport(borderLeftWidthStyle.element, styleWindow),
    "border-left-width is in view.");

  info("Highlight the computed font-size declaration in the rule view.");
  const fontSize = rules[2].textProps[0].editor;

  info("Wait for the view to scroll to the property.");
  onHighlightProperty = view.once("scrolled-to-element");
  isHighlighted = view.highlightProperty("font-size");
  await onHighlightProperty;

  ok(isHighlighted, "font-size property is highlighted.");
  ok(isInViewport(fontSize.element, styleWindow), "font-size is in view.");

  info("Highlight the pseudo-element's color declaration in the rule view.");
  const color = rules[0].textProps[1].editor;

  info("Wait for the view to scroll to the property.");
  onHighlightProperty = view.once("scrolled-to-element");
  isHighlighted = view.highlightProperty("color");
  await onHighlightProperty;

  ok(isHighlighted, "color property is highlighted.");
  ok(isInViewport(color.element, styleWindow), "color property is in view.");
});

function isInViewport(element, win) {
  const { top, left, bottom, right } = element.getBoundingClientRect();
  return top >= 0 && bottom <= win.innerHeight && left >= 0 && right <= win.innerWidth;
}
