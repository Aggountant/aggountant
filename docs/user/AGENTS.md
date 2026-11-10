# docs/user/ — the user guide

This directory is the user guide: how a person runs their own company's
bookkeeping with aggountant and an AI agent. `fi/` is the Finnish original
and `en/` its English counterpart; both describe the same tool with the same
facts. The reader is a small-business owner or bookkeeper, not a developer,
and reads the guide on GitHub as plain Markdown, not as a glasspad space.

## The guide changes with the code

The guide is a permanent document and describes the tool as it is now. When a
change alters a command, a flag, an input field, an error code the guide names,
or the behaviour a chapter describes, the same commit updates the guide in
every language that exists. A guide that lags one release behind teaches users
commands that fail, and they have no way to tell which half is stale.

Before changing a statement, check it against the code and
`aggountant <command> --help`. Describe only what exists. Where a step is still
manual, or a command exists but cannot yet be used for real work (for example
because no live connection is behind it), say so plainly in the chapter instead
of leaving it out: the user needs to know what they still do themselves.

## Examples and privacy

Every example uses a fictional company and synthetic figures: Esimerkki Oy,
Business ID 1234567-1, contact Maija Meikäläinen, fictional IBANs and amounts.
Never use a real company's data, figures, counterparties or personal data, and
no real machine names or home-directory paths. The repository is public.

## Writing

Plain Finnish bookkeeping language in `fi/`. Natural English in `en/`, not a
literal translation: Finnish statutory and service names (ALV, OmaVero,
Tulorekisteri, tilinpäätös, Y-tunnus, Ilmoitin.fi) and the Finnish account
names of the chart stay in Finnish with an English explanation where they
first appear, because that is what the user sees in the authorities' services
and in aggountant's output. For each task say what the user does, what the
agent does, and what the user approves and why. Name internal
mechanisms only when the user meets them on screen (an error code, a gate
name in a command); architecture vocabulary stays out. The glossaries
(`fi/09-sanasto.md`, `en/09-glossary.md`) define only terms a user actually
meets.

The chapter files are numbered in reading order and each language's
`README.md` is its table of contents. Both languages have the same chapters
under the same numbers, so they can be compared side by side; file names are
in the chapter's own language. A new chapter, or a change to a fact, a command
or a limitation, goes into every language in the same commit.
