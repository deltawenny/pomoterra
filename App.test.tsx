import { expect, open, test } from "nsh/test";

test("shows only the Timer tab and the initial focus session", async () => {
  const app = await open(".");
  expect(app.texts()).toContain("Timer");
  expect(app.texts()).toContain("45 : 00");
  expect(app.texts()).toContain("Stay Focused");
  expect(app.texts()).toContain("1 / 4");
  expect(app.texts()).toContain("Sessions");
});

test("starts, pauses, and resets the timer", async () => {
  const app = await open(".");
  await app.press("Start timer");
  await app.press("Pause timer");
  await app.press("Reset timer");
  expect(app.texts()).toContain("45 : 00");
  await app.press("Start timer");
  await app.press("Pause timer");
});

test("skips between focus and break sessions", async () => {
  const app = await open(".");
  await app.press("Skip session");
  expect(app.texts()).toContain("05 : 00");
  expect(app.texts()).toContain("Take a Break");
  await app.press("Skip session");
  expect(app.texts()).toContain("45 : 00");
  expect(app.texts()).toContain("2 / 4");
});

test("toggles completion announcements and note editing", async () => {
  const app = await open(".");
  await app.press("Mute announcements");
  await app.press("Enable announcements");
  await app.press("Edit note");
  await app.press("Save note");
  expect(app.texts()).toContain("Stay Focused");
});
