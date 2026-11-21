# Result: close-year-archive-write-error

The archive is written right after the fiscal year is sealed and before the
`seal-year` success is journalled, so the note records a failed write
(`archive not written: …`). A failed write does not fail the seal: the year is
sealed and the run completes. `close-year`'s `message` then says the year is
sealed and the books are correct but `archive/<year>/tilinpaatos.json` is
missing (with the recorded reason), and `next_command` is `close-year resume`,
which rewrites the archive when this run sealed the year and no archive file
is there; a rewrite that fails again is an error naming the path. The checks
use `is_file()`, so an obstruction at the path is not mistaken for an archive.
Tests: `a_failed_archive_write_is_reported_and_resume_rewrites_it`,
`a_failing_archive_rewrite_on_resume_is_an_error`. The guide and the skill
say what happens.

Landed with `close-year-next-command`; review and assessment are in that
issue's directory.
