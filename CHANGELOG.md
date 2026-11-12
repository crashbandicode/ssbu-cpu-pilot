# Changelog

## 0.2.4 (2026-11-11), core 0.1.4

- P1's L+R+X stands down while Online Deluxe's window shows a page with rows
  (Settings, Window Options, Plugins, the CPU Pilot page), where the press
  belongs to the window: on the CPU Pilot page's P1 row Deluxe asks for the
  step itself, which is now the press's one step; on any other row (for
  example You or NetProfile on Settings) the CPU level is left alone. With the
  window hidden, or on a page without rows (Performance, Debug), where
  Deluxe's L+R+X does nothing, the pad steps as before.
- The page comes from Deluxe's `ssbu_online_deluxe_overlay_state_v1` export
  (Deluxe 0.10.5 or later; `DELUXE_ROWS_PAGE`). It is looked up at boot and,
  after a miss, again on the worker thread at most once a second; hooks only
  call the found address. Without the export (an older Deluxe, or none) there
  is no rows page, and 0.2.3's rule stands: in battle the pad steps and
  Deluxe's request for the current choice does not step again.
- The install line reports `deluxe_overlay=found` or `missing`.
- Core 0.1.4: the `deluxe` module (the export's struct, flags and symbol,
  `rows_page`, `pad_steps`, `request_steps`, `Retry`).

## 0.2.3 (2026-11-11), core 0.1.3

- P1 as CPU in the Quickplay and Elite Smash waiting-room practice, routed as
  in Training (`Scope::WaitingPractice`, `online::route`). When a match is
  found, the practice's session is released without touching its objects and
  the battle arms afresh (`online::same_scene`).

## Earlier

0.2.2 admits the console's Quickplay battle online; 0.2.1 shows the CPU level
on the P1 row and steps it with L+R+X from the row; 0.2.0 puts the rows on
their own Deluxe page; 0.1.0 is the first release. See the commit history and
[docs/EDEN-RESULTS.md](docs/EDEN-RESULTS.md).
