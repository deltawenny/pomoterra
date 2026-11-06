import { expect, open, test } from "nsh/test";

// Run with POMOTERRA_DB_PATH=:memory: to isolate each app instance.
test("creates and edits a payable profile, then deletes it", async () => {
  const app = await open(".");
  await app.press("Config");
  await app.press("Add profile");
  await app.clear("Profile name");
  await app.type("Profile name", "Client work");
  await app.press("Save profile name");
  expect(app.texts()).toContain("Client work");
  await app.press("Payable");
  await app.clear("Hourly rate");
  await app.type("Hourly rate", "45.50");
  expect(app.value("Hourly rate")).toBe("45.50");
  await app.press("Use color #a855f7");
  expect(app.value("Profile color hex")).toBe("#a855f7");
  await app.press("Delete profile");
  await app.press("Confirm delete");
  expect(app.texts()).toContain("Default");
});

test("history filters show an empty state without completed sessions", async () => {
  const app = await open(".");
  await app.press("Skip session");
  await app.press("History");
  expect(app.texts()).toContain("Focus history");
  expect(app.texts()).toContain("0 focus sessions in the last year");
  expect(app.texts()).toContain("0 sessions · 0h 0m");
  await app.press("History: Month");
  await app.press("Reverse date order");
  expect(app.texts()).toContain("SESSIONS · OLDEST FIRST");
  await app.press("History: All");
  expect(app.texts()).toContain("No completed focus sessions in this period. Finish a focus session to start your history.");
});

test("shows Config navigation and the initial focus session", async () => {
  const app = await open(".");
  expect(app.texts()).toContain("Timer");
  expect(app.texts()).toContain("25 : 00");
  expect(app.texts()).toContain("Stay Focused");
  expect(app.texts()).toContain("1 / 4");
  expect(app.texts()).toContain("Sessions");
});

test("opens Config, restores defaults, and returns to Timer", async () => {
  const app = await open(".");
  await app.press("Config");
  expect(app.texts()).toContain("RULES");
  expect(app.texts()).toContain("25 mins");
  expect(app.texts()).toContain("5 mins");
  expect(app.texts()).toContain("15 mins");
  expect(app.texts()).toContain("4 rounds");
  expect(app.texts()).toContain("SPECIAL BREAKS");
  const time = "Special break 1 start time (HH:MM)";
  const duration = "Special break 1 duration in minutes (1–180)";
  await app.type(time, "09:30");
  await app.type(duration, "10");
  await app.press("Timer");
  await app.press("Config");
  expect(app.value(time)).toBe("09:30");
  expect(app.value(duration)).toBe("10");
  await app.press("Restore Default");
  expect(app.value(time)).toBe("");
  expect(app.value(duration)).toBe("");
  await app.press("Timer");
  expect(app.texts()).toContain("25 : 00");
  expect(app.texts()).toContain("1 / 4");
});

test("rule sliders update the timer and round counter", async () => {
  const app = await open(".");
  await app.press("Config");
  await app.drag("label:Stay focused", { x: 1000, y: 0 });
  expect(app.texts()).toContain("120 mins");
  await app.drag("label:Session rounds", { x: 1000, y: 0 });
  expect(app.texts()).toContain("10 rounds");
  await app.press("Timer");
  expect(app.texts()).toContain("120 : 00");
  expect(app.texts()).toContain("1 / 10");
});

test("starts, pauses, and resets the timer", async () => {
  const app = await open(".");
  await app.press("Start timer");
  await app.press("Pause timer");
  await app.press("Reset timer");
  expect(app.texts()).toContain("25 : 00");
  await app.press("Start timer");
  await app.press("Pause timer");
});

test("skips between focus and break sessions", async () => {
  const app = await open(".");
  await app.press("Skip session");
  expect(app.texts()).toContain("05 : 00");
  expect(app.texts()).toContain("Take a Break");
  await app.press("Skip session");
  expect(app.texts()).toContain("25 : 00");
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
