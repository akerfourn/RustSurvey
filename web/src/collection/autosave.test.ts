import assert from "node:assert/strict";
import { test } from "node:test";
import { createAutosave, type AutosaveState } from "./autosave.ts";

function setup() {
  let now = 0;
  let serial = 0;
  const timers = new Map<number, { at: number; action: () => void }>();
  const attempts: number[] = [];
  const autosave = createAutosave(
    {
      now: () => now,
      schedule: (action, delay) => {
        const id = ++serial;
        timers.set(id, { at: now + delay, action });
        return () => {
          timers.delete(id);
        };
      },
    },
    () => {
      attempts.push(now);
    },
  );
  function advance(duration: number): void {
    const end = now + duration;
    while (true) {
      const next = [...timers].sort((left, right) => left[1].at - right[1].at)[0];
      if (next === undefined || next[1].at > end) break;
      now = next[1].at;
      timers.delete(next[0]);
      next[1].action();
    }
    now = end;
  }
  return { autosave, attempts, advance };
}

function changed(editVersion: number): AutosaveState {
  return { editVersion, attemptedVersion: 0, dirty: true, enabled: true };
}

test("test_autosave_should_wait_for_two_seconds_without_edits_and_never_save_a_clean_form", () => {
  const { autosave, attempts, advance } = setup();
  autosave.update({ ...changed(0), dirty: false });
  advance(20000);
  assert.deepEqual(attempts, []);
  autosave.update(changed(1));
  advance(1500);
  autosave.update(changed(2));
  advance(1999);
  assert.deepEqual(attempts, []);
  advance(1);
  assert.deepEqual(attempts, [23500]);
});

test("test_autosave_should_bound_the_wait_during_continuous_typing", () => {
  const { autosave, attempts, advance } = setup();
  autosave.update(changed(1));
  for (let version = 2; version <= 7; version += 1) {
    advance(1500);
    autosave.update(changed(version));
  }
  advance(999);
  assert.deepEqual(attempts, []);
  advance(1);
  assert.deepEqual(attempts, [10000]);
});

test("test_autosave_should_cancel_a_pending_timer_when_a_manual_save_begins", () => {
  const { autosave, attempts, advance } = setup();
  autosave.update(changed(1));
  advance(1900);
  autosave.update({ ...changed(1), attemptedVersion: 1, enabled: false });
  advance(20000);
  autosave.update({ ...changed(1), attemptedVersion: 1, dirty: false });
  advance(20000);
  assert.deepEqual(attempts, []);
});

test("test_autosave_should_wait_for_an_active_write_and_then_save_the_newer_edits", () => {
  const { autosave, attempts, advance } = setup();
  autosave.update(changed(1));
  advance(2000);
  autosave.update({ ...changed(1), attemptedVersion: 1, enabled: false });
  advance(100);
  autosave.update({ ...changed(2), attemptedVersion: 1, enabled: false });
  advance(3000);
  assert.deepEqual(attempts, [2000], "no concurrent write may be scheduled");
  autosave.update({ ...changed(2), attemptedVersion: 1 });
  advance(0);
  assert.deepEqual(attempts, [2000, 5100]);
});

test("test_autosave_should_not_repeat_a_failed_attempt_without_another_edit", () => {
  const { autosave, attempts, advance } = setup();
  autosave.update(changed(1));
  advance(2000);
  autosave.update(changed(1));
  advance(30000);
  assert.deepEqual(attempts, [2000]);
  autosave.update(changed(2));
  advance(2000);
  assert.deepEqual(attempts, [2000, 34000]);
});

test("test_autosave_should_stop_when_access_is_blocked_or_an_acknowledgement_is_uncertain", () => {
  const { autosave, attempts, advance } = setup();
  autosave.update(changed(1));
  advance(1000);
  autosave.update({ ...changed(1), enabled: false });
  advance(30000);
  assert.deepEqual(attempts, []);
});

test("test_autosave_should_cancel_all_timers_when_the_form_is_unmounted", () => {
  const { autosave, attempts, advance } = setup();
  autosave.update(changed(1));
  advance(1000);
  autosave.dispose();
  autosave.update(changed(2));
  advance(30000);
  assert.deepEqual(attempts, []);
});
