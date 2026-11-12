#!/usr/bin/env python3
"""Tests of scripts/qa/cold_play_check.py (task Q-COLD).

Each test builds an evidence folder by hand, in the shape that tests/browser/cold-session.js writes, and
runs the checker on it. The good folder must pass; each other folder breaks ONE thing and must fail the
named check, and only that check. The node test tests/browser/cold-play.test.js runs the checker on folders
that the real session code wrote (the round trip), so the two files cannot drift apart without a red.

    python3 -B tests/qa/test_cold_play_check.py
"""
import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CHECKER = ROOT / "scripts/qa/cold_play_check.py"
STATEMENT = "\n".join([
    "GOAL: win the match", "GOAL_SHOT: 0002-click-p200.png",
    "FIRST_ACTION: click the big button", "FIRST_ACTION_SHOT: 0002-click-p200.png",
    "WORKED: the panel opened", "WORKED_SHOT: 0002-click-p1000.png",
]) + "\n"


class Folder:
    """A good evidence folder, as data. A test changes the data, then calls write()."""

    def __init__(self):
        self.session = {"session_version": 1, "limit_ms": 600000, "statement_after_ms": 60000, "sample_ms": 500,
                        "viewport": {"width": 1280, "height": 720}, "lock_marker": "qcold-test-1"}
        self.timeline = []  # (t_ms, kind, fields), written in this order
        self.shot_bytes = {}
        self.statement = STATEMENT
        self.sound = {"released_at_ms": 80000, "reason": "end", "final": True,
                      "sampling": {"interval_ms": 500, "samples": 160, "failures": 0, "bridge_missing": 0, "last_error": None},
                      "context_states": [], "records": []}
        self.end = {"reason": "end", "t_ms": 80000, "limit_ms": 600000, "calls": 3, "first_click_ms": 2140, "statement_stored": True}
        self.write_statement = True
        self.skip_files = set()
        self.timeline_ready()

    def shot(self, name, t_start, target):
        name = f"{name}.png"
        self.shot_bytes[name] = f"PNG {name}".encode()
        self.timeline.append((t_start + 30, "shot", {"file": name, "target_ms": target, "t_start_ms": t_start, "t_done_ms": t_start + 30}))

    def inp(self, n, cmd, args, t_start, t_end, counts=True, down=None):
        self.timeline.append((t_end, "input", {"n": n, "cmd": cmd, "args": args, "t_start_ms": t_start, "t_down_ms": down,
                                                  "t_end_ms": t_end, "counts_as_input": counts}))
        self.shot(f"{n:04d}-{cmd}-p200", t_end + 200, t_end + 200)
        self.shot(f"{n:04d}-{cmd}-p1000", t_end + 1000, t_end + 1000)

    def timeline_ready(self):
        self.timeline.append((0, "ready", {}))
        self.shot("0000-start", 0, 0)
        self.inp(1, "wait", {"seconds": 1}, 100, 1100, counts=False)
        self.timeline.append((2140, "first_click", {"t_down_ms": 2140}))
        self.inp(2, "click", {"x": 640, "y": 360, "button": "left"}, 2100, 2260, down=2140)
        self.inp(3, "key", {"code": "KeyF"}, 3300, 3420, down=None)
        self.timeline.append((70000, "statement_stored", {}))  # sha256 is set in write()
        self.timeline.append((70001, "sounds_released", {"reason": "statement", "final": False, "records": 0}))
        self.timeline.append((80000, "sounds_released", {"reason": "end", "final": True, "records": 0}))
        self.timeline.append((80001, "end", {"reason": "end"}))

    def events(self, kind):
        return [item for item in self.timeline if item[1] == kind]

    def write(self, directory):
        directory = Path(directory)
        (directory / "shots").mkdir(parents=True)
        (directory / "internal").mkdir()
        (directory / "session.json").write_text(json.dumps(self.session))
        events, inputs = [], []
        times = [item[0] for item in self.timeline]
        for seq, (t_ms, kind, fields) in enumerate(self.timeline, 1):
            event = {"seq": seq, "t_ms": max(t_ms, events[-1]["t_ms"] if events else 0), "kind": kind, **fields}
            if kind == "shot":
                data = self.shot_bytes[fields["file"]]
                event.update(sha256=hashlib.sha256(data).hexdigest(), bytes=len(data))
                if fields["file"] not in self.skip_files:
                    (directory / "shots" / fields["file"]).write_bytes(data)
            if kind == "statement_stored":
                event.update(sha256=hashlib.sha256(self.statement.encode()).hexdigest(), bytes=len(self.statement))
            events.append(event)
            if kind == "input":
                inputs.append(event)
        (directory / "events.jsonl").write_text("".join(json.dumps(event) + "\n" for event in events))
        (directory / "inputs.jsonl").write_text("".join(json.dumps(event) + "\n" for event in inputs))
        if self.write_statement:
            (directory / "statement.txt").write_text(self.statement)
        if self.sound is not None:
            (directory / "sound-records.json").write_text(json.dumps(self.sound))
        if self.end is not None:
            (directory / "end.json").write_text(json.dumps(self.end))
        return directory


def run(directory, *extra):
    result = subprocess.run([sys.executable, "-I", "-B", str(CHECKER), str(directory), *extra], capture_output=True, text=True)
    return result.returncode, result.stdout, result.stderr


class CheckerTest(unittest.TestCase):
    def check(self, folder, *extra):
        with tempfile.TemporaryDirectory() as tmp:
            directory = folder.write(Path(tmp) / "evidence")
            return run(directory, *extra)

    def assertFails(self, folder, check_id, *extra):
        code, out, err = self.check(folder, *extra)
        self.assertEqual(code, 1, out + err)
        failed = [line.split()[1] for line in out.splitlines() if line.startswith("  FAIL")]
        self.assertEqual(failed, [check_id], out)
        return out

    def test_good_folder_passes_and_lists_machine_and_manual_checks(self):
        code, out, err = self.check(Folder())
        self.assertEqual(code, 0, out + err)
        self.assertIn("MACHINE CHECKS", out)
        for check_id in ("E1", "E1b", "E2", "E2b", "L1", "L2", "C2a", "C3a", "C3b", "C3c", "C3d", "C3e", "C3f", "C3g", "C3h", "S1", "F5a", "F5b"):
            self.assertRegex(out, rf"  (PASS|INFO) {check_id} ", check_id)
        self.assertIn("MANUAL CHECKS", out)
        for check_id in ("C0", "C1", "C2", "C3m", "C4", "C5"):
            self.assertRegex(out, rf"  TODO {check_id} ")
        self.assertIn("This is not PERSON-READY.", out)
        self.assertIn("MATCH BY HAND: key KeyF at 3.3 s; the last screenshot before it: 0002-click-p1000.png", out)

    def test_json_output(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Folder().write(Path(tmp) / "evidence")
            code, out, err = run(directory, "--json")
        data = json.loads(out)
        self.assertEqual((code, data["failed"]), (0, []))
        self.assertTrue(any(row["check"] == "F5a" and row["status"] == "PASS" for row in data["machine"]))
        self.assertEqual([item["check"] for item in data["manual"]], ["C0", "C1", "C2", "C3m", "C4", "C5"])

    # ---- C3: the statement gate -------------------------------------------------------------

    def test_no_statement_fails_c3a_and_c3c(self):
        folder = Folder()
        folder.timeline = [item for item in folder.timeline if item[1] != "statement_stored"]
        folder.write_statement = False
        folder.timeline = [(t, k, {**f, "reason": "end-without-statement"} if k == "sounds_released" else f) for t, k, f in folder.timeline]
        code, out, _ = self.check(folder)
        self.assertEqual(code, 1)
        failed = [line.split()[1] for line in out.splitlines() if line.startswith("  FAIL")]
        self.assertEqual(failed, ["C3a", "C3c"], out)

    def test_statement_after_release_fails_c3c(self):
        folder = Folder()
        stored = folder.events("statement_stored")[0]
        folder.timeline.remove(stored)
        # the sound records were released first (70001), the statement came after (70002)
        folder.timeline.insert(folder.timeline.index(folder.events("sounds_released")[0]) + 1, (70002, "statement_stored", {}))
        out = self.assertFails(folder, "C3c")
        self.assertIn("before the sound records were released", out)

    def test_statement_before_60_s_fails_c3d(self):
        folder = Folder()
        old = folder.events("statement_stored")[0]
        folder.timeline[folder.timeline.index(old)] = (30000, "statement_stored", {})
        folder.timeline.sort(key=lambda item: item[0])
        self.assertFails(folder, "C3d")

    def test_no_first_click_fails_c3d(self):
        folder = Folder()
        folder.timeline = [item for item in folder.timeline if item[1] != "first_click"]
        self.assertFails(folder, "C3d")

    def test_input_between_the_60_s_mark_and_the_statement_fails_c3e(self):
        folder = Folder()
        statement = folder.events("statement_stored")[0]
        at = folder.timeline.index(statement)
        folder.timeline.insert(at, (66000, "input", {"n": 9, "cmd": "key", "args": {"code": "KeyA"}, "t_start_ms": 65900, "t_down_ms": None, "t_end_ms": 66000, "counts_as_input": True}))
        folder.shot("0009-key-p200", 66200, 66200)
        folder.shot("0009-key-p1000", 67000, 67000)
        # the two shots belong before the statement too
        for item in folder.timeline[-2:]:
            folder.timeline.remove(item)
            folder.timeline.insert(at + 1, item)
        out = self.assertFails(folder, "C3e")
        self.assertIn("inputs [9]", out)

    def test_wait_between_the_mark_and_the_statement_is_allowed(self):
        folder = Folder()
        at = folder.timeline.index(folder.events("statement_stored")[0])
        folder.timeline.insert(at, (66000, "input", {"n": 9, "cmd": "wait", "args": {"seconds": 1}, "t_start_ms": 65000, "t_down_ms": None, "t_end_ms": 66000, "counts_as_input": False}))
        folder.shot("0009-wait-p200", 66200, 66200)
        folder.shot("0009-wait-p1000", 67000, 67000)
        for item in folder.timeline[-2:]:
            folder.timeline.remove(item)
            folder.timeline.insert(at + 1, item)
        code, out, err = self.check(folder)
        self.assertEqual(code, 0, out)

    def test_do_not_know_fails_c3g(self):
        for text in ("GOAL: I do not know", "GOAL: Don't know what to do", "GOAL: unknown", "GOAL: none"):
            folder = Folder()
            folder.statement = STATEMENT.replace("GOAL: win the match", text)
            self.assertFails(folder, "C3g")

    def test_missing_or_duplicate_part_fails_c3f(self):
        folder = Folder()
        folder.statement = STATEMENT.replace("WORKED_SHOT: 0002-click-p1000.png\n", "")
        out = self.assertFails(folder, "C3f")
        self.assertIn("WORKED_SHOT is missing", out)
        folder = Folder()
        folder.statement = STATEMENT + "GOAL: again\n"
        self.assertFails(folder, "C3f")

    def test_statement_text_that_differs_from_the_record_fails_c3b(self):
        folder = Folder()
        with tempfile.TemporaryDirectory() as tmp:
            directory = folder.write(Path(tmp) / "evidence")
            (directory / "statement.txt").write_text(STATEMENT + "GOAL_NOTE: changed later\n")
            code, out, _ = run(directory)
        self.assertEqual(code, 1)
        self.assertIn("FAIL C3b", out)

    def test_a_part_with_no_screenshot_file_fails_c3h(self):
        folder = Folder()
        folder.statement = STATEMENT.replace("0002-click-p1000.png", "0042-click-p200.png")
        out = self.assertFails(folder, "C3h")
        self.assertIn("WORKED_SHOT", out)
        folder = Folder()
        folder.statement = STATEMENT.replace("GOAL_SHOT: 0002-click-p200.png", "GOAL_SHOT: ../evidence/session.json")
        self.assertFails(folder, "C3h")

    def test_a_screenshot_taken_after_the_statement_fails_c3h(self):
        folder = Folder()
        statement = folder.events("statement_stored")[0]
        at = folder.timeline.index(statement)
        folder.shot("0004-wait-p200", 75000, 75000)
        late = folder.timeline.pop()
        folder.timeline.insert(at + 2, late)
        folder.statement = STATEMENT.replace("WORKED_SHOT: 0002-click-p1000.png", "WORKED_SHOT: 0004-wait-p200.png")
        out = self.assertFails(folder, "C3h")
        self.assertIn("taken after the statement", out)

    # ---- F5: the sound records --------------------------------------------------------------

    def sounds(self, seen, sound_id="pkg:beep", category="effects"):
        return [{"sound_id": sound_id, "category": category, "play_id": i, "player_entity": 100 + i, "request_frame_index": i, "first_seen_ms": ms}
                for i, ms in enumerate(seen, 1)]

    def test_four_equal_sound_ids_in_60_s_fail_f5a(self):
        folder = Folder()
        folder.sound["records"] = self.sounds([10000, 20000, 30000, 40000])
        out = self.assertFails(folder, "F5a")
        self.assertIn("pkg:beep: 4 times from 10.0 s to 40.0 s", out)

    def test_three_in_60_s_and_four_in_75_s_pass(self):
        for seen in ([10000, 20000, 30000], [0, 25000, 50000, 75000], [5000, 15000, 25000, 66000]):
            folder = Folder()
            folder.sound["records"] = self.sounds(seen)
            code, out, _ = self.check(folder)
            self.assertEqual(code, 0, out)

    def test_different_ids_do_not_add_up(self):
        folder = Folder()
        folder.sound["records"] = self.sounds([10000, 20000], "a") + self.sounds([30000, 40000], "b")
        self.assertEqual(self.check(folder)[0], 0)

    def test_an_alert_sound_before_the_first_input_fails_f5b(self):
        folder = Folder()
        folder.sound["records"] = self.sounds([500], "pkg:forge_alert", "alerts")
        out = self.assertFails(folder, "F5b")
        self.assertIn("pkg:forge_alert at 0.5 s", out)
        folder = Folder()
        folder.sound["records"] = self.sounds([500], "pkg:ping", "alerts")  # the category says alert, the id does not
        self.assertFails(folder, "F5b")

    def test_an_alert_sound_after_the_first_input_passes(self):
        folder = Folder()
        folder.sound["records"] = self.sounds([2500], "pkg:forge_alert", "alerts")
        self.assertEqual(self.check(folder)[0], 0)

    def test_a_sound_that_is_not_an_alert_before_the_first_input_passes(self):
        folder = Folder()
        folder.sound["records"] = self.sounds([500], "pkg:click", "effects")
        self.assertEqual(self.check(folder)[0], 0)

    def test_sound_records_that_cannot_be_trusted_fail_s1(self):
        broken = {
            "not final": lambda s: s.update(final=False),
            "too few samples": lambda s: s["sampling"].update(samples=3),
            "bridge missing": lambda s: s["sampling"].update(bridge_missing=160),
            "many failures": lambda s: s["sampling"].update(failures=100),
        }
        for name, change in broken.items():
            folder = Folder()
            change(folder.sound)
            code, out, _ = self.check(folder)
            self.assertEqual(code, 1, name)
            self.assertIn("FAIL S1", out, name)

    def test_no_sound_file_fails_s1(self):
        folder = Folder()
        folder.sound = None
        code, out, _ = self.check(folder)
        self.assertEqual(code, 1)
        self.assertIn("FAIL S1", out)

    # ---- Limits ----------------------------------------------------------------------------------

    def test_a_limit_above_1500_s_fails_l1(self):
        folder = Folder()
        folder.session["limit_ms"] = 1501000
        self.assertFails(folder, "L1")

    def test_a_session_that_ran_past_its_limit_fails_l1(self):
        folder = Folder()
        folder.session["limit_ms"] = 60000
        folder.end.update(reason="limit", t_ms=80000)
        self.assertFails(folder, "L1")
        folder = Folder()
        folder.session["limit_ms"] = 75000
        folder.end.update(reason="limit", t_ms=80000)  # 5 s late: inside the grace of 10 s
        self.assertEqual(self.check(folder)[0], 0)

    def test_a_session_with_no_end_fails_l1(self):
        folder = Folder()
        folder.end = None
        folder.timeline = [item for item in folder.timeline if item[1] != "end"]
        self.assertFails(folder, "L1")
        folder = Folder()
        folder.end.update(reason="page-closed")
        self.assertFails(folder, "L1")

    def test_a_missing_lock_marker_fails_l2_unless_skipped(self):
        folder = Folder()
        folder.session["lock_marker"] = None
        self.assertFails(folder, "L2")
        code, out, _ = self.check(folder, "--no-lock-check")
        self.assertEqual(code, 0, out)

    def test_the_wrapper_receipt_is_checked_when_given(self):
        for receipt, passes in (({"status": "ok", "hold_s": 700.0, "limit_s": 1800}, True), ({"status": "ok", "hold_s": 1900.0, "limit_s": 1800}, False),
                                ({"status": "failed: hold limit", "hold_s": 1800.0, "limit_s": 1800}, False), ({"status": "ok", "hold_s": 700.0, "limit_s": 600}, False)):
            with tempfile.TemporaryDirectory() as tmp:
                directory = Folder().write(Path(tmp) / "evidence")
                path = Path(tmp) / "lock-receipt.json"
                path.write_text(json.dumps(receipt))
                code, out, _ = run(directory, "--receipt", str(path))
            self.assertEqual(code, 0 if passes else 1, f"{receipt}\n{out}")

    # ---- E1, E2, C2a -----------------------------------------------------------------------------

    def test_a_missing_screenshot_file_fails_e2(self):
        folder = Folder()
        folder.skip_files.add("0002-click-p200.png")
        code, out, _ = self.check(folder)
        self.assertEqual(code, 1)
        self.assertIn("FAIL E2 ", out)
        self.assertIn("0002-click-p200.png: missing", out)

    def test_a_changed_screenshot_fails_e2(self):
        folder = Folder()
        with tempfile.TemporaryDirectory() as tmp:
            directory = folder.write(Path(tmp) / "evidence")
            (directory / "shots/0002-click-p200.png").write_bytes(b"other")
            code, out, _ = run(directory)
        self.assertEqual(code, 1)
        self.assertIn("bytes differ from the record", out)

    def test_a_late_screenshot_fails_e2b(self):
        folder = Folder()
        for index, (t, kind, fields) in enumerate(folder.timeline):
            if kind == "shot" and fields["file"] == "0002-click-p200.png":
                folder.timeline[index] = (t + 900, kind, {**fields, "t_start_ms": fields["t_start_ms"] + 900, "t_done_ms": fields["t_done_ms"] + 900})
        folder.timeline.sort(key=lambda item: item[0])
        out = self.assertFails(folder, "E2b")
        self.assertIn("call 2: +200 ms screenshot started 900 ms off", out)

    def test_an_early_screenshot_and_a_missing_one_fail_e2b(self):
        folder = Folder()
        for index, (t, kind, fields) in enumerate(folder.timeline):
            if kind == "shot" and fields["file"] == "0003-key-p200.png":
                folder.timeline[index] = (t, kind, {**fields, "t_start_ms": fields["t_start_ms"] - 150})
        self.assertFails(folder, "E2b")
        folder = Folder()
        folder.timeline = [item for item in folder.timeline if not (item[1] == "shot" and item[2]["file"] == "0003-key-p1000.png")]
        code, out, _ = self.check(folder)
        self.assertEqual(code, 1)
        self.assertIn("call 3: no +1000 ms screenshot", out)

    def test_a_gap_in_the_event_numbers_fails_e1(self):
        folder = Folder()
        with tempfile.TemporaryDirectory() as tmp:
            directory = folder.write(Path(tmp) / "evidence")
            lines = (directory / "events.jsonl").read_text().splitlines()
            del lines[5]
            (directory / "events.jsonl").write_text("\n".join(lines) + "\n")
            code, out, _ = run(directory)
        self.assertEqual(code, 1)
        self.assertIn("FAIL E1 ", out)

    def test_inputs_that_differ_from_the_event_log_fail_e1b(self):
        folder = Folder()
        with tempfile.TemporaryDirectory() as tmp:
            directory = folder.write(Path(tmp) / "evidence")
            lines = (directory / "inputs.jsonl").read_text().splitlines()
            (directory / "inputs.jsonl").write_text("\n".join(lines[:-1]) + "\n")
            code, out, _ = run(directory)
        self.assertEqual(code, 1)
        self.assertIn("FAIL E1b ", out)

    def test_a_key_with_no_name_fails_c2a(self):
        folder = Folder()
        for index, (t, kind, fields) in enumerate(folder.timeline):
            if kind == "input" and fields["cmd"] == "key":
                folder.timeline[index] = (t, kind, {**fields, "args": {"code": ""}})
        self.assertFails(folder, "C2a")

    # ---- A folder that cannot be read --------------------------------------------------------------

    def test_an_unreadable_folder_exits_2(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assertEqual(run(Path(tmp) / "none")[0], 2)
            self.assertEqual(run(tmp)[0], 2)
            folder = Folder()
            directory = folder.write(Path(tmp) / "evidence")
            (directory / "events.jsonl").write_text("{not json}\n")
            code, out, err = run(directory)
            self.assertEqual(code, 2)
            self.assertIn("events.jsonl line 1 is not JSON", err)

    def test_the_checker_never_prints_person_ready_as_a_result(self):
        code, out, _ = self.check(Folder())
        self.assertNotRegex(out, r"RESULT: PERSON-READY")


if __name__ == "__main__":
    unittest.main()
