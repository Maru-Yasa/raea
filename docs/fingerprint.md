# Fingerprint: the plan

Shane, 2026-11-12: "build a framework within raea called Fingerprint ... a taxonomy data structure and an embedding
model that can run on a personal device, that could be easily trained on device, to best understand if a video (its
audio and video and whatever else we can put together into this) is good or not ... train through comments or notes
or a post-mortem process the agent can run ... and if you have a team, you can distribute those embedding models."

Shane, 2026-11-20, once the videoform plan had landed: "lets make sure ... we consolidate fingerprints, videoform and
everything into one big thing."

This is the one plan for everything raea knows about what good work is. It covers what a fingerprint holds and where
each part came from, how one is chosen for a piece, how it checks a piece and how it learns. It then covers each part
in turn: the axes, the kind of video (the videoform) and judging on a rubric. It ends with the measurements behind each
part and the build order, with a confidence for each step. The rask tasks are all in
[fingerprint.feature](fingerprint.feature):
- FP0-FP15: the fingerprint, with FP15 the consolidation;
- DR5-DR9: notes, moved from direction.feature;
- VF1-VF13: the videoform, planned by @wackyza in raea #207 and moved from videoform.md and videoform.feature.

The reference design for the axes is [earshot](https://github.com/Leonxlnx/earshot) (MIT), a local sound critic for
agents. It is the closest working system to what the axes do for sound. We take its methods, not its code (see
"earshot, the reference").

## One fingerprint

A fingerprint is everything raea knows about what good work is, for one person, one team, one project or one kind of
video. Until 11-20 that knowledge sat in six places, each with its own file, command and reader. Each was right about
its own part, but three things went wrong because they were apart:
- an agent read three files and ran two commands to learn what it would be held to;
- a note taught one of them and not the others;
- a team shared some of them and not the rest.

So they are now one thing: a fingerprint is a folder of parts, and every part is optional.

| Part | What it holds | File | What reads it |
|---|---|---|---|
| what it's for | `about`, `applies` (kinds and tags), `extends` | fingerprint.ts | `which` (FP14c) |
| taste | standing lines in words: the person's "always" and "never" | taste.ts, taste/KIND.ts | every agent before work (`raea context`) |
| rules | a line with the check that holds it: one of raea's by name with its values, or a `.ts` check; its level; where it came from | rules.ts | `raea check` |
| axes | the taxonomy: each axis's words, its head, and the person's weighting over them | fingerprint.ts, words.f16, heads.safetensors | `raea fingerprint score` |
| ranges | each measure's range across work the person admires, or across a videoform's references | profiles/ | the critique, and range checks |
| rubric | categories, their words and the pass, for work judged on sheets (a 3D scene's review) | rubric.ts | `raea fingerprint judge` |
| videoform | a kind of video: its context, its references read once, its vocabulary, shot kinds and structure | fingerprint.ts's `videoform`, vocabulary.ts, read/ | following it, and checking against it |
| evidence | every label the parts learn from, the person's and other eyes' | evidence.jsonl, evidence.f16 | training every part above |
| report | what each part knows, how sure it is, and what it can't judge | report.json | the person, and agents |

Three things hold for every part, as they already held for the axes:
1. **Measured checks are the arbiter** wherever a thing can be measured. A learned part is advice until it earns its
   gate.
2. **A note teaches the fingerprint that judged the piece** (FP14d), whichever part it lands in, and moves up only when
   the person moves it.
3. **Agents get words, moments and rules,** never a score or a threshold.

## Today's six, and where each goes

| Today | Where it lived | In the one fingerprint |
|---|---|---|
| taste.ts | ~/.raea/taste/taste.ts, with a file for each kind of work (motion, games, sound, ui); a workspace's taste.ts; read by `raea context` and `raea check --taste` | the taste of the person's own fingerprint and of the workspace's. A line with one of the six check forms (text-hold, text-above, text-below, text-margin, loudness, still-max) is also a rule |
| controllers' `rule` fields | controllers/ID.ts in a workspace or a project (DR4), edited on the app's Controllers page | rules.ts in the workspace's or the project's fingerprint. Controllers keep what they are for: settings and workflows |
| notes | notes/ROUND/nN.ts, through `raea notes take, ls, post, check, keep` (DR5-DR9) | still the way a person's words come in and are restated. An ask's check, and whether it passes, is evidence in the fingerprint that judged the piece. `raea notes keep` makes the check a rule there |
| Fingerprint | ~/.raea/fingerprints/NAME, and a workspace's or a project's .raea/fingerprints (FP0-FP14) | the core the others join. Its folder, `which`, inheritance, evidence, sharing and gate now serve every part |
| videoform | the plan in raea #207 (VF1-VF13), not built | a fingerprint's `videoform` section, as #207 planned it, with no command of its own. Its mined rules go to rules.ts rather than a controller |
| raea judge | apps/judge: a frozen rubric (rubric.rs) judged by clef-omni, the person's `raea judge eye` scores, and `calibrate` | the rubric, edited like any part. The person's eye scores are evidence, the model's verdicts are another eye (FP6d), and "calibrated" is the gate (FP2d) |

These don't move:
- **brief.ts** stays the piece's own: who it's for, what they should believe, its modes, and the references and look for
  this piece.
- **raea check's floor** (text, contrast, safe areas, freezes, flashing, loudness) is raea's, the same for everyone. A
  fingerprint's rules add to it and never replace it.
- **Controllers** keep settings and workflows: delivery specs, who reviews what, the tools agents get.
- **The video-decompiler skill's** REFERENCE.md and `vd pin` stay the skill's, for a product film made from code.

## The folder

```
~/.raea/fingerprints/NAME/          the person's own; a workspace's or project's is under its .raea/fingerprints/
  fingerprint.ts     about, applies, extends; the axes; the encoders' pins; videoform: { context, references, template }
  taste.ts           the standing lines (taste/KIND.ts: lines read only for that kind of work)
  rules.ts           each rule: its words, its check and values, its level, where it came from
  rubric.ts          categories, their words and the pass, for work judged on sheets
  vocabulary.ts      a videoform's terms, written by the person's AI
  words.f16          the axes' and the vocabulary's words as vectors
  heads.safetensors  the learned heads
  profiles/          ranges
  read/              a videoform's per-shot rows, its structure and a still for each shot kind
  evidence.jsonl     every label, with evidence.f16 holding its columns
  report.json        what each part knows and can't judge
```

The .ts files are the person's to edit, typed by `raea/fingerprint`. raea writes the rest. The person's own
fingerprint is the one their account defaults to (`me` on a new machine). A workspace's or a project's lives in its
folder and travels with its repo.

## Which fingerprint judges a piece

`which` (FP14c) chooses one fingerprint for a piece, and every part now comes from that choice:
- it looks in the project, then the workspace, then the person's own;
- at each level, the fingerprint whose `applies` matches the piece most closely wins.

Taste and rules gather up the `extends` chain, as axes and evidence already do. Say a project's fingerprint extends the
workspace's, which extends the person's own. An agent then reads all three's taste, and the piece is held to all three's
rules. Where two rules hold the same check, the nearer one wins.

That's what raea check did by hand, from three taste files (the person's, the kind's and the workspace's) and the
controllers' rule fields. Now it's one walk. `raea fingerprint which PATH` prints the fingerprint it chose and why, then
each taste file and each rule it brings, with where it came from. A kind's taste file (taste/motion.ts) is read when the
brief's modes need it, as ~/.raea/taste's kind files are today.

## One check

`raea check PROJECT` holds a piece to raea's floor and to the fingerprint that judges it, in one list of findings:
- **The fingerprint's rules,** its own and its parents'. These are:
  - taste's check forms;
  - what used to be controller rule fields;
  - notes kept as rules;
  - a videoform's mined rules.

  Wherever a rule can run on the composition it does, in milliseconds: order, slot lengths, coverage, shot lengths,
  pauses, caption places.
- **Its ranges,** on the render's measures: loudness, light, colour and pace (the profile's critique, FP6e).
- **A videoform's structure and shot kinds.** Only shots that changed are embedded again (VF11).
- **Its axes' flags** (FP3a's moments): a warning from a gated axis and a note from an ungated one, so an advisory head
  never fails a render.
- **A rubric's verdict,** for work judged on sheets.

Each finding names the part and the rule it came from, for example "rules.ts: captions clear the button band, kept from
note 3/n2". Each is routed as raea-app-check-cause routes every finding (raea #205):
- a fault the footage brought is a note;
- a finding an earlier export also had says so;
- a round goes only to what another render can change.

`raea fingerprint score` stays the axes' own reading, with its sheet and timeline. Its flags are the same findings,
counted once.

## One learning loop

Every label lands in the fingerprint that judged the piece (FP14d). It moves up to a parent only when the person moves
it. Sources, strongest first:
1. a mark or a box on a defect note (FP11);
2. a note restated into asks (DR5), with each ask's check failing or passing (DR6). The reading is a label on its axis
   (FP5), and the check is a candidate rule;
3. an approval, a picture lock, a publish or a golden reference: positives over every un-noted span;
4. a pick between real options, a hand edit's before and after, or the person's blind pick in a videoform's test (VF13):
   pairs;
5. work the person admires, and a videoform's references: ranges (FP6e, VF4);
6. the person's yes or no on a videoform's strongest rules (VF8), and their scores on a rubric's categories (`judge
   eye`);
7. version pairs: a noted moment that changed and wasn't noted again. These are weak pairs;
8. damage variants (bootstrap);
9. the post-mortem's answers;
10. clef's, Kimi's or another model's readings and verdicts, the rubric's model judge among them. These are kept as
    another eye and never counted as the person's (FP6d).

A check earns its way up as a head does. A note's check that holds across rounds (DR7) is offered as a rule (DR8), in
the fingerprint that judged or in the one above it if the person says so. A taste line the person repeats as a
correction is offered as a check (the taste loop, docs/craft-system.md J).

## The axes

The axes are a fingerprint's learned part. They are not a fine-tuned embedding model but a **taxonomy of axes**: the things that can be good or wrong
about a piece, such as a soft picture, dead motion, noise under the voice or a cut that drags. Each axis has the
**words** people use for it and **one small head** over **frozen, pinned encoders that run on the person's own
device**. On top of the axes sit the person's **weighting** across them, an optional **profile** (ranges taken from
work they admire) and the **evidence ledger** that every head was trained from.

The frozen encoders together are the **Edit Embedding Zoo** (Shane, 11-13: the standard name, in the docs, the app
and anything we publish): SigLIP 2 for pictures, CLAP for how it sounds, YAMNet for sound events and their timing,
EmbeddingGemma 2 for text and Depth Anything 3 for depth. They are pinned, run on the person's device and are the same
for every fingerprint. A fingerprint is what a person trains over the zoo: its axes' heads. The zoo itself never
changes.

On day one, an axis's words already give it a zero-shot score. Each label the person adds retrains its head in
milliseconds. Whatever can be measured exactly, such as loudness, silence gaps, frozen frames or one thing passing
through another, is a check rather than a learned head. A team shares evidence, never media, and each machine
retrains from it.

The hard part is turning notes into labels, not the model. On Shane's real notes, picture and sound alone can't
predict where he will leave a note. They do learn a named axis from 4 to 32 examples, and some axes from no examples
at all.

## The stack

```
media ─► columns (once, at ingest, cached by content hash)
           picture: SigLIP 2 per frame at 4 fps (pooled vector; 14x14 patch grid for flagged or marked frames)
           change:  frame-to-frame SigLIP distance          look: luminance histogram, saturation, contrast, sharpness
           sound:   CLAP audio, 2 s windows, 1 s hop          YAMNet, 0.48 s frames (event classes and timing)
           text:    EmbeddingGemma 2 for scripts, briefs and other text (a still gets the picture columns)
           things:  a render's layers or EdgeTAM masks, depth from its AOV or Depth Anything 3, and the camera
       ─► checks (no model): loudness, silence gaps, clipping, flash, freeze, sync to cut, floating, passing through
       ─► axis heads: word head (zero-shot) → learned head (logistic / Bradley-Terry) → patch head (heatmap)
       ─► person's weighting over axis scores          ─► profile ranges from reference work
       ─► summary · JSON · sheet PNG · timeline · watch lane
```

| part | model | size on disk | runs at score time | cost for a 90 s render |
|---|---|---|---|---|
| picture columns | SigLIP 2 base p16-224 (image tower) | 95 MB int8, 372 MB fp32 | yes | 17 s on ort's CPU path (47 ms a frame); MLX on the Mac once MX5 lands; never Core ML's GPU |
| sound columns | CLAP htsat-unfused (audio tower) | 117 MB fp32 (34 MB q8 copy, off by default) | yes | about 3 s (33 ms a 2 s clip) |
| sound events | YAMNet | 16 MB | yes | under 1 s |
| change and look | none | none | yes | negligible |
| words, scripts, briefs | EmbeddingGemma 2 | 841 MB q8, the Library's copy when it has one | only for text files; a still gets the picture columns | 2.3 s a window |
| axis words → vectors | CLAP text tower (502 MB fp32), SigLIP 2 text tower | | **no**: only when an axis's words change; the vectors are stored in the fingerprint | |
| heads, weighting, profile | ours | 7 KB a head | yes | milliseconds |
| things | EdgeTAM, Depth Anything 3 Small | 37 MB, 51 MB (both already in perception) | only for thing-level axes and checks | about 86 ms a frame for EdgeTAM |

Only CLAP is a new download. Its fp32 audio and text towers come to 619 MB. The q8 copies (161 MB) match fp32 only to
cosine 0.998, and differ between ONNX Runtime builds, so fp32 is the default (RAEA_CLAP_DTYPE=q8 picks the copies). Every encoder runs in Rust:
- **ONNX Runtime** (ort, through raea-sys-ml) is the path on every platform and the reference. YAMNet already runs on
  it, CLAP's audio and text towers join it, and CLAP's tokenizer is the `tokenizers` crate raea-sys-speech already
  uses.
- **MLX** (raea-sys-mlx) is for SigLIP 2 on the Mac, the one heavy column. On this Mac, ort's CPU path costs 47 ms a
  frame and Core ML's GPU computes SigLIP wrong. SigLIP is already a named candidate for mlx.feature MX5, and it moves
  there once its MLX path matches ONNX fp32.
- **CLAP stays on ort.** At 33 ms a clip on the CPU there's nothing to gain from rewriting its HTSAT network in MLX.

transformers.js (onnxruntime-node underneath, the same .onnx files) was used only to measure. Everything runs inside
the feed host, which stays alive, so the models load once.
earshot's CLI reloads its models on every call (20-40 s) and has to steer agents to its MCP server for that reason.
`raea fingerprint` talks to the running host instead. Columns are cached by content hash and step version, which is
safer than earshot's cache key of path, size, modification time and sampled blocks.

**Used only for labelling and training, never at score time:** the Director reading notes into labels, EdgeTAM
growing a click into a region, the open-vocabulary grounder (gated, FP13b), the damage and red-team generators, the
text towers, and the gated LoRA trainer (FP10).

**Training stays plain Rust until FP10.** The encoders are frozen and inference only, so they stay on ort. A head is
a logistic fit on a few hundred to a few thousand rows of at most 1,024 values; L-BFGS in plain Rust fits one in
milliseconds and needs no framework. A training framework earns its place only with FP10's adapter, which back-propagates
through SigLIP 2; then MLX's autograd and Burn (Metal, CUDA and CPU backends) are compared on PyTorch parity and on
what the person's device can run.

### Axes and their heads

An axis is declared in TypeScript beside `taste` (`fingerprint({...})` from "raea/brief"). It has:
- a name and the words people use for it;
- a modality: picture, change, look, sound, events or text (a script's, brief's or page's words);
- a scope: moment, shot or the whole piece;
- a polarity;
- optionally, the check it maps to.

raea ships a base taxonomy drawn from Shane's notes: glitch and continuity, sound, pace, camera, cut, light and look,
realism and detail, interaction, design and type, story. A person or team adds axes. Creative directions ("could be a
garden door") are not axes. They go to the brief and the agent, never into a head.

Each axis uses the best head it has earned:
1. **A word head** (from day one): the similarity between the window and the axis's words, as a z-score against a
   background vocabulary of about 40 ordinary descriptions. This is earshot's calibration. CLAP serves sound axes,
   SigLIP 2's text tower serves picture axes and EmbeddingGemma 2 serves text axes. A word head is always advice: it never gates.
2. **A learned head**: logistic on the axis's own modality columns, standardised with the encoder's shipped
   statistics, plus Bradley-Terry when the axis has pairs. It takes over from the word head once it beats it on
   held-out evidence.
3. **A patch head** (picture axes): logistic on SigLIP 2's patch tokens, trained from local defects (FP12a) and,
   once there are enough, marks (FP12b). It says *where* in the frame.

An axis that a check measures exactly, such as dropout or loudness, reports the check, and its head only adds what
the check can't see.

## How the axes train

1. **Day 0.** The base taxonomy's words give every axis an advisory score. `raea fingerprint bootstrap` damages
   windows of the person's own approved work (blur, dark, glow, flash, freeze, noise, dropout, clipping) and trains
   the mechanical axes from those labels by construction (FP0b). No placeholder footage is used.
2. **Every label is an evidence row**: the media's content hash and span, the region or layer, axis, polarity,
   weight, who gave it, its source, and the window's columns (fp16, about 4 KB). The heads retrain after each row,
   in milliseconds.
3. **Sources** are the ones listed in "One learning loop", strongest first. For the axes, a note is read by the
   Director into {kind, axis, span, layer} before it is a row (FP5).
4. **The gate.** A head is advice until it has 16 labels of each polarity from 3 or more projects, a held-out by
   project AUC of 0.8 or better, and a catch rate of 0.8 or better on its red-team cheats. A gated head reports
   bands, as earshot's verdicts do: strong, plausible or weak, each with the precision measured on that head's own
   held-out evidence.
5. **The person's weighting** is a logistic fit over the axis scores, from their own labels (FP0c).
6. **The post-mortem.** At a project's end, the agent scores every version, finds where the fingerprint is least sure
   or disagrees most with the person's history, and asks at most ten yes/no questions with the moment playing
   (`raea show --question`). Each answer is an evidence row.
7. **Encoder upgrades.** A row whose media is present is re-embedded and its head retrained. A row without media keeps
   the old pin until it's labelled again.
8. **Fine-tuning the encoder** is gated. It is tried only for an axis still under its bar with 300 labels or more after
   its own columns were added (glow is the likely first), as a LoRA adapter on SigLIP 2 trained on the device through
   MLX. It is kept only if it raises held-out AUC by 0.05.

## A kind of video: the videoform

@wackyza planned the videoform on 11-19 (raea #207). It answers the person's ask that day: an agency or creator "making
the same type of video over and over again can drop in a bunch of those videos, and raea codifies that style so the
user can then drop a bucket of raw footage and raea with great accuracy and intent edits the raw footage into the exact
style it has learned ... and it all needs to happen FAST". Then: "the point is to embed them, store the embeds, and be
able to repeat the style without having to go back to the references themselves". And a videoform is "a kind of
fingerprint, flowing into what has already been built".

An agency or a creator makes the same kind of video again and again: a listing walkthrough, a recipe short, a podcast
clip, a product unboxing. They give raea five to ten finished videos of that kind, from their own drive or by link, and
a line of context ("real estate walkthroughs for agents' Reels"). raea reads them and writes down what makes them that
kind of video: its **videoform**. From then on raea cuts new raw footage to it, and checks every pass against it,
quickly.

The consolidation changed two things in #207's plan and nothing else:
- a videoform's rules are the fingerprint's rules.ts, not a controller's rule fields;
- `raea fingerprint` makes it and adds to it, so there is no `raea videoform` command.

### Why a videoform is read from approved work, not written from notes

A style written by hand from a person's corrections records only what went wrong once. What was right from the start
is never said, so it's never written down. Pacing is the plainest case: if the first cut's voice was tight, nobody
leaves a note about it, the written style never mentions it, and the next cut made from that style can be slack while
satisfying every line of it. The approved videos already hold the answer, in numbers: how long the pauses are, how
long the shots are, how fast the words come, where the cuts fall. A videoform read from them carries what was approved,
not only what was corrected. Notes still matter. They teach what the numbers can't (DR6-DR8, FP5), on top of it.

### Decisions taken

- **The name is videoform.** It is the `videoform` section of fingerprint.ts, made and fed by `raea fingerprint`
  (decided 11-20; #207 had its own command).
- **A videoform is a kind of fingerprint,** flowing into what's built. It's a fingerprint (FP14) that `extends` the
  person's own and adds what a kind of video needs: its context, its references, its vocabulary, its structure and its
  rules. Everything Fingerprint already does applies to it: finding the one that applies, inheriting evidence, being
  taught by notes, being shared with a team and keeping its evidence when an encoder upgrades.
- **What ships runs on shipped models, on the person's machine.** Reading, embedding, storing and building new videos
  from a videoform use only the models raea ships, plus at most one or two light additions. Nothing in it needs a paid
  model API. Pro and Studio add raea's own pool of clef checks and content intelligence on top (decision 12), and it
  only ever speeds the work up.
- **The domain vocabulary comes from the person's own AI.** The agent already in raea (Claude, Kimi, any of them) turns
  the context line into the things to look for. That adds no new bill and no shipped domain packs.
- **What stays the same across the references is the videoform; what varies is the content.** A rule's strength is
  its consistency across the references. Every videoform is checked by building it from all references but one and
  testing it on the one left out.
- **A reference is read once, and never needed again.** Its embeddings and measures are what the videoform keeps, and
  they're enough to follow it and check against it forever after. Following and checking never open a reference, so
  they stay fast, and the references can be deleted, offline or behind a dead link without the videoform noticing.
- **Checks start from the composition.** Most of a videoform's rules are measured on what raea knows it put where,
  before any render. Embeddings run only on shots that changed.

### What raea already has, and what this builds on

| Already in raea | What it gives a videoform | Gap this plan fills |
|---|---|---|
| Fingerprint: `fingerprint.ts` with `about`, `extends`, `applies` kinds and tags; `which` finds one by project, workspace and account; a child inherits its parent's evidence, its own counted more; notes teach the one that judged (FP14a-e) | the home: a videoform is one of these | the fields a kind of video needs |
| Fingerprint's frozen encoders (SigLIP 2, CLAP, YAMNet, EmbeddingGemma 2, Depth Anything 3), columns cached by content hash, word and learned heads, evidence in f16 rows (FP1-FP4) | the per-shot vectors and measures, and where they're kept | no shot vocabulary, no structure |
| Fingerprint profiles: ranges from admired work, critique against them, a leave-one-out test (FP6e, `raea fingerprint profile`) | a videoform's ranges, and its validation | few measures (loudness, tilt, cut rate, motion, light, colour, sharpness) |
| Controllers' `rule` fields, a check by name or a `.ts` check on raea/check's world (DR4) | rules that `raea check PROJECT` and every agent hold work to | none written from references |
| Notes become checks, and climb to a fingerprint's rules (DR5-DR8, `raea notes keep`) | the path a correction takes to becoming a rule | a videoform's rules start from evidence, not notes |
| `raea check`: findings and the record (DR3); who can fix what, carried findings, one decode (raea-app-check-cause, PR #205) | the check every pass runs | a videoform's rules join it as more rules |
| `NAME.script.ts` (scenes, beats, lines, cut); `brief.ts` (`references`, `look`, `mode`, `deliver`) | a storyboard spec, and a brief that names references | no template made from references; references named, never read |
| `taste.ts`, six checked forms | the person's standing lines | the forms are fixed: mined rules go to rules.ts |
| Footage perception (`systems/footage`: shots, cuts, camera, faces, words on screen, scene labels, performance, facts.json) | what each reference shot is | read for raw footage, not for references |
| Transcription (Parakeet), beats, stems and voice activity (`raea song`, HTDemucs, Silero VAD), OCR | words, pauses, cuts against beats, music under voice, captions | not gathered into a videoform |
| Index and libraries (SigLIP over a person's media; turbopuffer in the cloud) | finding footage by meaning; vector storage for teams | — |
| video2raea sound, img2raea | reverse engineering a piece, with answer keys from our own films | — |
| The video-decompiler skill (raea-skills): one admired film taken apart into a written dossier (`REFERENCE.md`), `vd pin` writing a film's numbers as targets | holding a piece to a reference, and measured targets | one reference, read by hand; product films made from code, not footage cuts |
| raea's URL cache (`io/net`): a direct file URL fetched once into a cache | reading a media link | it saves the file; no platform pages, playlists or channels |

### A videoform is a kind of fingerprint

`fingerprint.ts` gains an optional `videoform` section. A fingerprint with one is a videoform. It's found by `which`
like any other (its `applies` names the kinds and tags of piece it judges), it `extends` the person's fingerprint, so
their general taste carries in, and it's kept and shared where fingerprints are (`.raea/fingerprints/NAME`, a
project's, a workspace's or the person's own).

```
.raea/fingerprints/listing-walkthrough/
  fingerprint.ts      about, extends, applies, axes; and videoform: { context, references, template }
  rules.ts            its mined rules, each with its strength, beside any the person or a note added
  vocabulary.ts       what to look for, written by the person's AI from the context
  profiles/           the videoform's ranges (FP6e's profile, measures added)
  read/               the references' per-shot rows, in the f16 format Fingerprint's evidence uses; the structure; the report
  evidence.jsonl      as any fingerprint: the person's notes, picks and verdicts on pieces it judged
```

Each part of a videoform flows into the part of raea that already does that job:

| A videoform's... | Flows into | So that |
|---|---|---|
| ranges (pace, pauses, shot lengths, loudness, colour, captions) | its own profile (FP6e) | `raea fingerprint score --profile` critiques a cut against them, and leave-one-out is already built |
| the taste taught later | its own evidence and heads (FP2, FP14b-d) | a note on a piece it judged teaches it, never the person's general fingerprint unless they move it up |
| rules (coverage, order, event rules, limits) | the fingerprint's rules.ts | `raea check PROJECT` runs them, and they're edited on the app's Fingerprints page like any rule |
| structure (slots, their order and lengths) | a `NAME.script.ts` template, which `videoform.template` names | a project made to the videoform starts from it, and agents plan from it as from any storyboard |
| references | `brief.ts`'s existing `references` shape, `{ target, note }`, with the source kept by content hash | a reference is named the way a brief already names one |

`raea fingerprint new NAME --videoform "…" REF...` makes one. `raea fingerprint which` on a
piece names the videoform that judges it and why, and `raea context` prints it with the brief.

### Bringing videos in, with nothing downloaded

- **From the local drive.** A file or a folder (`raea fingerprint add NAME ~/Videos/listings/`), or a library's files, is
  read where it is. No copy is made.
- **From a URL, streamed.** `raea fingerprint add NAME URL...` reads the video over the network in one pass and keeps only
  what was measured:
  - **A direct link to a file** (an `.mp4` on a server, S3, a share link that serves the file) is decoded straight from
    the network.
  - **A platform page** (YouTube, TikTok, Instagram, Vimeo, X), a playlist or a creator's channel with `--last N`
    isn't a video file: it's a page with a player, and the video sits behind it as short-lived, signed stream URLs,
    often in segments. A resolver (yt-dlp, Unlicense, fetched on first use as raea fetches FFmpeg on Windows) turns
    the page into those stream URLs, and raea streams them. Nothing is written to disk.
- **One pass.** A streamed video can't be sought cheaply, so every reader takes its share of the one decode, as raea
  check's picture rules now do. Shot bounds come from the same pass, and the per-shot reads hold a shot's frames until
  it ends.
- **What's kept.** Each video's readings (below), by content hash of the decoded stream, with its source URL, title,
  creator and date. The video itself is not kept. A video already read is recognised by its hash and never read twice.
- **Rights.** Once per account, before the first reference is read from a URL, raea asks the person to confirm they have
  the right to use what they add (their own work, a client's, or licensed), says that a videoform keeps measurements and
  vectors and never the video, and that a platform's terms are theirs to keep. The answer is kept with the account.
- **Limits.** Login-only and private links are refused, with the reason given. Streams are read at 1080p at most;
  reading never needs more.

### The resolver, and keeping it current

A platform page is resolved by **yt-dlp** (Unlicense): the most widely used and most actively kept page resolver, with
extractors for over a thousand sites and releases every week or two. raea uses it only to resolve, never to download:
`yt-dlp -J URL` (a playlist or channel: `--flat-playlist`, `--playlist-end N`) gives each video's formats as JSON, with
their stream URLs and the headers they need, and raea picks the best stream at 1080p or under and decodes it itself.

Sites change their pages often, and an old resolver stops working within weeks, so it's kept current by raea, not by
the person:
- **Fetched, not installed.** The official standalone binary for the platform (`yt-dlp_macos`, `yt-dlp_linux`,
  `yt-dlp.exe`) is fetched on first use into `~/.raea/bin`, as raea fetches FFmpeg on Windows, and checked against the
  release's published SHA-256 sums before it runs.
- **Pinned, then moved forward.** Each raea release pins the yt-dlp version it was tested with. Between raea releases,
  raea checks yt-dlp's latest release at most once a day when a URL is added, fetches it, checks its sums, and keeps the
  one before it.
- **Updated when a site breaks.** When resolving fails with an extractor error, raea fetches the latest release and
  tries once more. If the new one fails where the old one worked, it goes back to the old one and says which site
  failed.
- **Said, not hidden.** `raea doctor` prints the yt-dlp in use, its date, and the last site that failed. A link that
  can't be resolved is refused with the site's reason, and the person can add the file from their drive instead.
- **Its needs.** Some sites' pages (YouTube's) make yt-dlp run JavaScript, for which it uses a JavaScript runtime on the
  machine. raea gives it one: yt-dlp is pointed at a runtime raea ships or fetches, and the bench checks it against
  each site raea names as supported.
- **Tested.** The test bench resolves one public video from each supported site each night, so a broken site is known
  before a person finds it.

### Reading a reference

Each video is read into rows by what raea already runs:
- footage perception: shots, cuts, camera, faces, words on screen, scene labels;
- Fingerprint's columns: SigLIP 2 per shot, CLAP and YAMNet over the sound;
- Parakeet's words, and the voice's pauses;
- beats and stems;
- OCR's caption boxes;
- depth, for shot scale.

All of it is keyed by content hash, as Fingerprint's columns are, so a reference read for one videoform serves every
videoform that uses it.

### What a videoform keeps, and why it's enough

Following and checking need four kinds of thing from the references, and a videoform keeps exactly those:

| Kept | What it is | What uses it |
|---|---|---|
| per-shot rows | each reference shot's SigLIP 2 vector (one pooled vector, a few frames averaged), its CLAP vector over the shot's sound, and its measures (length, scale, camera move, caption boxes, light and colour, words, pauses at its edges, its place in the video) | the shot vocabulary; matching new footage to shot kinds; the embedding checks |
| the vocabulary's vectors | each term's phrasings through the text towers, as word heads store them (FP2e) | naming shot kinds; finding domain things in new footage |
| the profile | the ranges of every measure across the references (FP6e) | the critique, and composition-first range checks |
| the structure and the rules | the slots with their lengths and shot kinds; the event and coverage rules with their strength | the template, and the rules in rules.ts that `raea check` runs |

Also kept: one small still for each shot kind (a thumbnail, a few kilobytes), so the person can see what a kind is when
confirming rules, and the evidence the person adds over time. The reference videos aren't kept.

**Sizes.** A videoform of ten one-minute references, about 50 shots each, keeps about 500 per-shot rows of about 3 KB
each (SigLIP 2's 768 values and CLAP's 512, in f16, with the measures): about 1.5 MB with its profile, structure, rules
and stills. That's small enough to load at once, keep in git with the project, and share with a team.

**Times, measured and held to.** Reading a reference is once only, at about real time or faster: one decode, every
reader on its share of it. Making the videoform's ranges, shot vocabulary, structure and rules from its rows takes
seconds. Following and checking read only what's kept: the composition-first checks in milliseconds, matching a new
shot to its kind in well under a millisecond against ~500 rows, and one SigLIP frame for each changed shot.

### When it doesn't hold together

- **Too few references.** Under five, every rule and range is reported as weak, and nothing gates on it. Leave one out
  needs at least five to mean anything.
- **References that aren't one kind of video.** If the references fall into two or more clear groups (by structure, by
  shot kinds, by pace), raea says so and offers a videoform for each, rather than averaging them into one that fits
  none.
- **A trait that varies.** A measure whose range across the references is wide, or a rule that holds in few of them, is
  a free choice, not part of the videoform. It's recorded as free, so the agent knows it may decide.
- **An outlier.** One reference far from the others on several measures is named ("number 7 runs twice as fast as the
  rest"), and the person decides whether it belongs.

### Adding and removing references

Adding a reference reads only the new one, then makes the ranges, shot vocabulary, structure and rules again from the
kept rows, in seconds. Removing one drops its rows, and the same. The person's evidence (their notes, picks and
verdicts) stays through both, as a fingerprint's does.

### When an encoder changes

A videoform records the encoders its rows came from (their pins, as `fingerprint.ts` does). A raea that ships a new
encoder keeps using the videoform's rows with the old encoder's pin, for matching and checking against it. The measures
(lengths, pauses, captions, light) don't depend on an encoder and carry over as they are. To move a videoform to the new
encoder, its references are read again (`raea fingerprint refresh NAME`, given the files or links). Until then it keeps
working as it was. This is the one time a reference is needed again, and it's the person's choice.

### Writing the videoform

1. **The vocabulary.** The person's AI expands the context line into `vocabulary.ts`:
   - places, then the things in each place, each with several phrasings;
   - near-misses (a kitchen against a dining room);
   - OCR patterns for text on screen (prices, addresses);
   - things to ignore.

   These become the videoform's word heads (FP2e), scored by SigLIP's and CLAP's text towers. raea reports what each
   did on the references: never fires, fires everywhere, confused with another, or a recurring shot no term names. The
   AI revises the vocabulary, and two or three rounds is usually enough.
2. **The ranges,** across the references, into the videoform's profile:
   - shot lengths, and cuts a minute;
   - the voice's pauses: inside a line, between lines, between sections;
   - words a minute, and cuts against beats and sentence ends;
   - loudness, and music under voice;
   - caption size, place and words on screen at once;
   - colour and grade;
   - talking head against cutaway.
3. **The shot vocabulary.** Every reference shot is clustered on its vectors and measures. Each cluster gets its terms
   from the vocabulary, its measured ranges (scale, move, length) and how many references have it.
4. **The structure.** The references are aligned by their run of shot kinds and their script's meaning, into slots
   (open, sections, close), each with its length range, its shot kinds and what's on screen. This becomes the script
   template.
5. **The event rules.** How often one thing comes with another across the references ("a punch-in when a number is
   said", "a close-up of the counters in every kitchen"). Those that hold in most references become rules in rules.ts,
   each stating how strongly it holds.
6. **The person confirms the strongest rules once,** on a contact sheet ("countertop close-up in every kitchen: 9 of
   10"). Each yes or no is evidence in the videoform, as any label is in a fingerprint.

### Following it

- `raea context` prints the videoform: its template, its rules and its ranges.
- New raw footage is read the same way, and each shot is matched to the videoform's shot kinds.
- Before any cut, the agent is told what the footage lacks: "no close-up of the counters, which every kitchen in the
  references has". That's a shot list for the person.
- The agent plans the cut to the template, slot by slot, and edits to the ranges (cut points, pauses, shot lengths).

### Checking it, fast

1. **On the composition, before any render, in milliseconds:**
   - order and slot lengths;
   - coverage;
   - shot lengths and pauses (the voice's cut points are in the composition);
   - caption places and sizes.

   These are rules.ts's rules, run by `raea check PROJECT` with the fingerprint's others (see "One check").
2. **On the render manifest and measures, in seconds:** loudness, light and colour against the profile (`raea
   fingerprint score --profile`).
3. **On embeddings, only for shots that changed:** each shot against its slot's shot kind (one SigLIP frame a shot);
   the run of shots against the structure.

The routing from raea-app-check-cause (PR #205) applies unchanged: the footage's own faults are notes, carried findings
say so, and a round goes only to what another render can change.

### Judging

Judging is for testing the build, set by set. The person's verdict is the judge; a second, automatic judge for the
test bench is deferred (see the end).

**The person's judgement (the one that decides).** For each test, raea shows the original and its own edit side by side,
unlabelled, in a random order (a feed artifact, or `raea show --question`). The person:
- picks which they'd post;
- scores pacing, shot choice, structure, graphics, sound and overall feel, 1 to 5;
- writes what's wrong in their own words.

Each verdict is kept in the test bench's record, and is also evidence in the videoform (a pick is a pair, FP6b; a note
is a label, FP5). The judging trains the videoform as well as grading it.

### The test sets

The person provides the sets: 8 to 10 videos each, from one creator or one format, with raw footage for a few where it
exists. Every test reads a set's references once, then runs only on what the videoform kept, so the tests also prove
that a videoform is enough without its references. For each set:
- **Leave one out:** a videoform built from all but one reference must hold the one left out (its rules pass, its
  measures fall in range) more often than other sets' videos do.
- **Tells sets apart:** each videoform must reject other sets' videos. This is profile FP6e's test, extended to rules
  and structure.
- **Edit to it:** where raw footage for a held-out reference exists, the agent edits it to the videoform, and the cut is
  set beside the original.
- **Judged:** by the person, as above.

The sets, their answers and their verdicts are kept as fixtures, so each step of the build is measured on all of them.

## Judging on a rubric

`raea judge` (apps/judge) judged a 3D scene's review sheets (`raea render --review DIR`) blind. clef-omni scored them on
a frozen rubric, fresh every round and blind to the code. The rubric had seven scores (composition, form, assembly,
materials, light, detail, world) and twelve yes/no defects. The pass was decided in code: no blocking defect, the lowest
score 6 or more and the mean 7 or more. Until the verdict was calibrated against the person's eye (`raea judge eye`,
`raea judge calibrate`), it was advice. In the one fingerprint:
- **The rubric is rubric.ts:** its scores and defects, the words each is asked in, and the pass. raea's frozen rubric is
  the default, unchanged, so today's verdicts hold.
- **`raea fingerprint judge DIR`** judges with the rubric of the fingerprint that judges DIR's project. `raea judge`
  keeps working for one release and prints the new name.
- **The person's eye scores** (`raea fingerprint judge eye DIR composition=4 ...`) are evidence rows on the rubric's
  scores. Calibration is the rubric's gate, as a held-out AUC is an axis's.
- **The model's verdict is another eye's:** kept and shown, never counted as the person's.
- **Each verdict keeps its rubric's version** (judge.json). "Done" (two passes in a row) and the stall compare rounds,
  and two rounds on different rubrics don't compare.

## The CLI

```
raea fingerprint new NAME [--about T] [--extends FP] [--kinds K] [--tags T]
                                                     a fingerprint: ~/.raea/fingerprints/NAME, or the project's or workspace's
       [--videoform "CONTEXT" SOURCE...]             ...that knows a kind of video, from 5 to 10 of the person's videos (VF2)
raea fingerprint add NAME SOURCE... [--last N]       a videoform's references: files, folders, links, a channel's last N (VF1)
raea fingerprint refresh NAME SOURCE...              a videoform's references read again for a new encoder (VF8c)
raea fingerprint which PATH [--rules]                the one that judges a piece, why, and the taste and rules it brings (FP15d)
raea fingerprint score FILE [--fp NAME]              summary first: the moments it expects a note on, each axis in the person's words
       [--profile P] [--json] [--sheet out.png] [--timeline]
raea fingerprint teach FILE --axis A --at T          a label from the CLI or an agent: --bad/--good, --span, --box x0,y0,x1,y1
raea fingerprint bootstrap [--from DIR]              damage variants of approved work train the mechanical axes
raea fingerprint profile REFS... --as NAME           ranges from work the person admires, checked leave-one-out
raea fingerprint judge DIR [--by clef|kimi|claude]   a review's sheets judged on the rubric (FP15g)
raea fingerprint judge eye DIR SCORE=N... [--pass yes|no]
                                                     the person's own scores, as evidence
raea fingerprint report [NAME]                       each part: what it knows, how sure, what it can't judge
raea fingerprint postmortem PROJECT                  the ten questions, then the report's change
raea fingerprint diff A B                            the axes two fingerprints disagree on most, with the moments
raea notes take | ls | post | check                  a person's words taken, restated as asks, posted back, checked (DR5-DR7)
raea notes keep ID --as project|workspace|me         a note's check kept as a rule in that fingerprint's rules.ts (DR8)
raea check PROJECT                                   raea's floor and the fingerprint, in one list (FP15e)
```

The verbs not shown are unchanged: read, approve, pair, use, share, move and things (the CLI page has them all). Old
names:
- `raea judge` runs `raea fingerprint judge` for one release and says so.
- `raea videoform` never ships.
- `raea check --taste FILE` reads a taste file as it always has.

The sheet is one PNG an agent or a person reads at a glance. It holds frames with the patch heatmaps over them, the
axis lanes, the spectrogram with flagged sounds boxed, and the checks' findings. In the app, the same scores are a lane
on the watch timeline. Agents get the axis, the moment and the person's words, never a number or a threshold.

## Teams

A fingerprint travels with the project or workspace that keeps it (Git + LFS, or raea's API), under that folder's
permissions. The whole folder goes:
- taste, rules and the rubric as text;
- the axes' heads and evidence;
- a videoform's kept rows, about 1.5 MB for ten one-minute references, never the references themselves.

Teammates share evidence rows and each machine retrains: pooling beat averaging the heads (FP0c). Each person's
weighting stays their own. Embeddings partly reveal content, so evidence never leaves the project's permissions, and
media never travels at all.

## Agents, and keeping them honest

The feed's agent runs `raea fingerprint score` on a cut before posting it, and either fixes each gating flag or says
why not. A fix is checked against the flag it claims to fix, and the axis's cheats are rerun.

earshot found the risk the hard way. Once a sound-making search optimised against its learned critics, those critics
stopped telling made sounds from real ones, while its model-free measurements still could. Our own cheats agree:
glow at half strength is caught only 0.20. So:
- measured checks are the arbiter wherever a thing can be measured;
- every learned axis carries red-team cheats;
- agents never see scores;
- like earshot's LIMITS.md, `report.json` says what each head can't judge.

Before an agent works, it reads the fingerprint that judges the project. `raea context` prints its taste, its rules in
words, and a videoform's template and ranges. The agent never reads heads or scores. A rule it can't meet is a finding
it answers, either by fixing it or by saying why not, never by editing rules.ts. A rule changes only when the person
changes it, or keeps a note as one.

## earshot, the reference

earshot is a CPU-only Python toolkit that gives agents ears. It runs CLAP (laion/larger_clap_general, Apache-2.0) and
AST (BSD-3) over a film's sound and returns a timeline, sync checks, a critique against reference films and a
realness score. It's honest about its measured limits. It covers sound only, and its runtime is Python and PyTorch,
which raea doesn't ship. We take six ideas:

| earshot | Fingerprint | spec |
|---|---|---|
| text-audio match calibrated as a z against a 322-term vocabulary, with verdict bands whose precision is measured on held-out recordings | word heads from the axis words, for sound (CLAP) and picture (SigLIP 2); bands on gated heads | FP2e, FP2d |
| `profile` reference films, `critique` against their ranges with timestamped fixes, checked leave-one-out (86% in range against a 71% baseline) | profiles from golden references and approved work: the positives that notes don't give (3 approvals in 205 notes) | FP6e |
| realness: real against code-synthesised sound, a small head on CLAP vectors, grouped CV AUC 0.92 held out by category and engine | measured, not shipped: a head on CLAP and YAMNet means, labelled by construction from raea's own generators (TokenSynth, SAO, Fish) against recordings under CC0 and CC BY, scored 0.90, 0.80 and 0.57 held out by generator, under the 0.85 it needed on all three, so there is no realness axis | FP2f |
| summary first, `--json`, an annotated `sheet` PNG, a `timeline` transcript | the same outputs from `raea fingerprint score` | FP3a, FP3c |
| a resident MCP server, because loading the models costs 20-40 s | everything in the feed host, models loaded once | FP3a |
| "model critics stop discriminating after search; model-free measurements still do"; a "cannot judge" list | checks as arbiter, red-team cheats per axis, limits in report.json | FP4b, FP4c, FP9a |

We don't take:
- its Python and PyTorch runtime;
- AST: YAMNet already serves event classes, and CLAP's heads match or beat YAMNet's on our defects;
- its larger CLAP checkpoint (larger_clap_general), until htsat-unfused falls short on a measured axis;
- its optional Gemini critique, which breaks the two-API-models rule;
- its cache key.

Its results are not ours to quote as Fingerprint's. Our numbers for the same ideas are FP0g below.

## What was measured (11-12, this Mac; spike in ~/films/fingerprint-spike)

The data:
- **Notes:** Shane's 205 pinned notes, with time and place, on 17 feed renders (Stay assemblies 18-23 and the launch
  film's passes b-l).
- **Videos:** those 17 renders, the five golden references and 11 other renders: 33 videos, 34 minutes.
- **Encoders:** SigLIP 2 base (ONNX fp32), YAMNet, EmbeddingGemma 2 (q8, through apps/app host/library/embed.ts) and
  CLAP htsat-unfused (Xenova's ONNX through the same transformers.js).

**FP0a. Notes alone don't say where the next note goes.** A logistic head scored 197 noted 2 s windows against 263
windows at least 3 s from any note:

| features | AUC held out by video | AUC held out by film |
|---|---|---|
| position in the video | 0.41 | 0.40 |
| cheap columns (brightness, contrast, motion, loudness) | 0.53 | 0.51 |
| SigLIP 2 | 0.64 | 0.40 |
| YAMNet | 0.54 | 0.46 |
| EmbeddingGemma 2, picture and sound | 0.64 | 0.53 |

Within a film, the heads score above chance because the same scenes recur across versions. Across films they're at
or below chance whichever encoder reads the windows, so the encoder isn't what's missing. The notes aren't good/bad
labels:
- about 98 name a defect, about 74 are directions (23 are both), and 3 are approvals;
- there is one note every 3-4 s of film;
- only 21 of 143 noted moments make a version pair.

**FP0b. A named axis is learned from a few examples.** Defects were put into real renders and the heads held out by
source. Each cell is AUC at k = 4 / 16 / 32 labelled windows a class:

| axis | own columns (SigLIP 2, change, YAMNet) | CLAP head | CLAP words only, no labels | EmbeddingGemma 2 |
|---|---|---|---|---|
| blur | 1.00 / 1.00 / 1.00 | | | 0.70 / 0.90 / 0.96 |
| too dark | 0.72 / 0.90 / 0.95 | | | 0.58 / 0.74 / 0.85 |
| glow | 0.55 / 0.69 / 0.80 | | | 0.54 / 0.65 / 0.73 |
| flash frame | 0.97 / 0.97 / 0.97 | | | |
| freeze | 1.00 / 1.00 / 1.00 | | | 0.56 / 0.66 / 0.77 |
| noise under the sound | 0.99 / 1.00 / 1.00 | 0.94 / 1.00 / 1.00 | **0.97** | 0.82 / 0.92 / 0.97 |
| sound drops out | 0.79 / 0.94 / 0.97 | 0.86 / 0.99 / 0.99 | 0.66 | 0.68 / 0.88 / 0.94 |
| clipped sound | 0.91 / 0.97 / 0.98 | 0.94 / 0.99 / 1.00 | **0.98** | 0.80 / 0.92 / 0.96 |

**FP0g. CLAP, measured 11-12.** CLAP ran on 2 s windows, 130 windows from 26 sources.
- **Words alone.** With no labels at all, the words catch noise and clipping at AUC 0.97 and 0.98. At the threshold
  that flags 10% of clean windows, they catch 0.94 and 0.95 of the damaged ones.
- **Dropout fails from words** (0.66): silence isn't a sound a description matches. So dropout is a check (a silence
  gap) plus a head, never a word head.
- **Cheats.** Noise at a third of the level is caught 0.48 from words (0.78 on 5 s windows), so word heads stay
  advice.
- **Learned heads.** CLAP heads match or beat YAMNet on every sound axis, most at k = 4.
- **Window length.** 5 s windows change little: words 0.99 / 0.60 / 0.98; heads at k = 4 0.98 / 0.76 / 0.98.
- **q8 copy.** It holds the numbers (words 0.99 / 0.67 / 0.97), with audio vectors at cosine 0.9975 to fp32.
- **Cost.** 33 ms a clip on the CPU.

**FP1a. The Rust columns match, measured 11-12.** raea-sys-columns runs every encoder on ONNX Runtime's CPU kernels
(raea-sys-ml), and its tests check it against the spike (systems/columns/src/tests.rs, run with --ignored):
- **SigLIP 2's image tower** against ONNX fp32 in Python on the same frames: cosine 1.000000.
- **CLAP's audio tower**, fp32, its log-mel front end in Rust: mean and worst cosine 1.00000 over 260 clips. The q8
  copy came to a worst cosine of 0.9978, a difference between ONNX Runtime builds rather than the front end (fp32
  against fp32 matches), so fp32 is the default.
- **CLAP's text tower**: 1.00000 over the 40 vocabulary descriptions, once padded as transformers.js padded the
  batch. The export takes no attention mask, so raea embeds each text alone, unpadded.

**FP0b's lessons on method.**
- Standardise columns. Unstandardised, SigLIP hid behind YAMNet's larger values: blur scored 0.75 at k = 32.
- Keep each axis on its own modality. A head given both modalities was worse at small k: dropout scored 0.68 at k = 4.

**FP0c. Taxonomy first, then the person.** Eight shared axis heads, followed by a person's 9 weights fit from 32 of
their labels, scored 0.84, 0.73 and 0.95 on three made-up tastes. One head fit straight on the embeddings from the
same labels scored 0.73 and 0.64. Two opposed tastes stayed apart (each ranks the other's labels 0.38 and 0.39). Three
teammates with 16 labels each scored 0.66 alone, 0.74 with their heads averaged, and 0.76 with their evidence pooled
and retrained.

**FP0d. Cost and cheats.**
- **Cost.** A head fits in 3.9 ms on 32 windows and 81 ms on 2,430, scores 2,430 windows in 1.6 ms, and takes 7 KB.
- **Cheats**, at the threshold that flags 10% of clean work:
  - a de-rez sharpened afterwards is still caught 0.99;
  - noise at a third of the level 0.85;
  - glow only 0.33, and 0.20 at half strength.

  SigLIP's pooled vector ignores a picture's look, so look axes get their own columns.

**FP0f. Marks teach where; pins don't.** A linear head on SigLIP 2's 14 x 14 patch tokens put local defects in real
frames. Its top patch fell inside the defect box (chance 0.10):
- 0.85-0.96 of the time from 8 frames marked with a click;
- 0.89-0.99 from 8 frames marked with a box;
- 0.23-0.62 with only "this frame is bad";
- local glow 0.44-0.62.

On Shane's 203 pinned notes, a head ranks 35% of a frame's patches above the one he clicked, while his average click
position ranks 16% above it. His pins sit near the middle of the frame (x 0.50, y 0.52). A pin anchors a note where
it's easy to read; it doesn't mark the thing.

**Encoders ruled out.**
- **PE-AV small** (Meta) is a ViT-L/14 at 336 px per frame and 3.4 GB. Shane, 11-12: "if it cant run locally on
  consumer hardware its not usable."
- **V-JEPA 2** carries a research licence.
- **7B grounded vision-language critics** (Q-Insight, VisualQuality-R1) fail both the consumer-hardware rule and the
  two-API-models rule.
- **YOLOE** is AGPL.

The literature agrees with FP0b and FP0c. Personal aesthetics work finds per-user linear heads on frozen
vision-language features better than prompting or LoRA (arXiv 2604.11374). Video aesthetics work keeps the encoder
frozen (VADB, arXiv 2510.25238). Rich Human Feedback (CVPR 2024) learns heatmaps from people's points, as FP12 will.

## Build order, with confidence

Built from 11-12 to 11-17:
- the axes' foundation: FP0, FP1, FP2a-e, FP3a, FP3c and FP4;
- learning from people: FP5a, FP6 and FP8;
- where, and many fingerprints: FP10a, FP11, FP12a and FP14;
- notes taken and restated, with a check for each ask: DR5 and DR6, on DR4's rules and world.

What's left, in order:

| phase | step | what | confidence | why |
|---|---|---|---|---|
| 0, one | FP15a-d | the folder's parts (taste and rules) read through `which` and its `extends` chain. Today's taste files and controller rule fields are read where they are, then moved by `raea doctor --fix` | high | every part exists and is read today; this is one reader in place of three |
| | FP15e | one check: the rules, ranges, a videoform's structure and the axes' flags in one list, each finding naming its part | high | raea check already gathers rules and routes findings (DR4, #205), and the axes' flags are FP3a's |
| | FP15f, FP15g | every label in one loop; the rubric as a part, judged through `raea fingerprint judge` | high | evidence rows (FP2c) and judge's eye scores exist; only their sources are new |
| | FP15h-j | one page in the app, one manual section and guide, and the same findings before and after the move | medium-high | the app's fingerprint page exists (FP14e); parity is the proof nothing was lost |
| 1, people | DR7-DR9 | rounds hold; a note's check climbs to a rule in the fingerprint; a check that can be gamed is found before it's trusted | medium-high | DR5 and DR6 built the asks and their checks |
| | FP5b, FP5c | notes read into labels, checked against the person; the proof that matters | medium | FP0a: notes don't label themselves |
| | FP7a, FP9a | the post-mortem; agents score before posting and can't game it | medium | they need FP5's labels and red-team sets |
| 2, a kind of video | VF1, VF1b | references from the drive or streamed from links, read once; the resolver kept current | high; medium for platform pages | decoding over the network is standard; platform pages depend on the resolver keeping up with the sites |
| | VF2, VF3, VF3b | the `videoform` section, made by `raea fingerprint new --videoform`; one pass reading references into the kept rows; the proof that what's kept is enough | high | FP14's machinery and every reader exist |
| | VF4 | ranges into the profile, measures added | high | FP6e computes ranges and leave-one-out already |
| | VF5-VF7 | the vocabulary with the person's AI; the shot vocabulary; the structure template | medium-high, medium, medium | word heads already score from text; aligning varied videos is the hard part |
| | VF8, VF8b, VF8c | mined rules into rules.ts with their strength, confirmed by the person; when it doesn't hold together; references added and removed, encoders upgraded | medium, medium-high, high | co-occurrence is plain; telling habit from style needs the person |
| | VF9-VF11 | following; the composition-first checks; embedding checks on changed shots | medium-high, high, medium | context, scripts and rules over the world exist (DR4) |
| | VF12, VF13 | the test bench, and the person's blind judging as evidence | high | the measures are defined above |
| 3, where and what | FP12b | pointing on real marks | low for glow, high for the rest | FP0f |
| | FP13a-c | things per shot; a note's words find its thing; geometry axes are checks | high, gated, high | both models are in perception; the grounder ships only if FP13b passes |
| measured | FP2f | realness axis for sound | not shipped | held out by generator, SAO foley scored 0.57 against a bar of 0.85 (TokenSynth 0.90, Fish 0.80) |
| | FP3b | the golden references stay clean, and a known bad cut doesn't | medium | needs FP5's labels |
| | FP10 | a LoRA adapter on SigLIP 2, or another device-sized encoder | low | only for an axis stuck after its own columns |

**The first proofs that matter** are FP5c for the axes and VF4 with VF12 for the videoform.
- **FP5c.** A fingerprint built from Shane's evidence on Stay assemblies 18-22 has to rank the moments he noted on
  assembly 23 above the rest at AUC 0.70 or better, held out by film. The axis it names has to match his words. The
  spike couldn't reach that bar with raw notes, and the axes' learning from notes stands on it. Without it, a person
  still gets word scores, checks and bootstrapped heads the day they install.
- **VF4 with VF12.** On at least five of the person's sets, each videoform, built leave one out, has to hold its
  left-out video's pacing and shot lengths and reject other sets' videos, more often than chance by a clear margin. If
  the numbers alone can't tell videoforms apart, nothing built above them will. Platform pages (VF1b) are built only
  once this passes.

## Decisions taken by default (Shane can overrule any)

1. **Patch grid storage.** Every frame stores its pooled vector. The 14 x 14 grid (300 KB a frame, about 108 MB for a
   90 s render) is kept only for flagged or marked frames and recomputed on demand.
2. **The note reader is the Director (Kimi).** It is checked on 50 of Shane's notes before notes train any head.
3. **Markup tools live in apps/app's review player.** The work is coordinated with the player's owner through the
   orchestrator.
4. **Teams share evidence rows, not heads.**
5. **CLAP joins the stack** (Shane, 11-12: "include clap if we can if its not too much work"). It is about 300 lines
   of Rust on ort: the 48 kHz log-mel front end and the audio session (200), and the text tower with its tokenizer
   (100), run only when an axis's words change. Both are checked against the spike's vectors at cosine 0.999.
6. **Taste stays words; rules are checks.** taste.ts holds lines for agents to read. A line becomes a rule only with a
   check, and the rule lives in rules.ts, so a check never hides in prose.
7. **Controllers lose `rule` fields.** Settings and workflows stay. An existing rule field is read as a rule of its
   level's fingerprint until `raea doctor --fix` moves it, and the Controllers page sends a rule to the fingerprint
   page.
8. **The kinds' taste files stay files.** taste/motion.ts in the person's fingerprint is read when the brief's modes
   need it, as today, rather than becoming a child fingerprint each.
9. **`raea judge` folds into `raea fingerprint judge`.** The rubric's scores, defects and pass become rubric.ts, with
   today's frozen rubric as the default. A rubric edit is versioned in each verdict.
10. **The videoform has no command of its own.** Its crates keep the names #207 gave them (raea-app-videoform,
    raea-sys-videoform), as the section's code. `raea fingerprint` hands its verbs to them, because raea-cli has no
    room for more.
11. **Nothing moves by surprise.** Old files are read where they are until `raea doctor --fix`, or the app, moves them.
    A check run before and after the move gives the same findings (FP15j).
12. **Pro and Studio include checks and content intelligence** (Shane, 11-20). Each month's pool (about 8,000 frames
    checked on Pro, ten times that on Studio) stops when it's spent and is never billed past. It pays for two things:
    - **clef-omni answering the fingerprint's questions** about a render's frames (`POST /v1/clef/ask`), so the agent
      reads answers in words and opens only the frames they flag. Its verdicts are another eye's (FP6d), and it serves
      the axes with no labels yet and the rubric's model judge.
    - **content intelligence for references** (`GET /v1/intel/...`): public posts, videos, transcripts and ads, to find
      a videoform's references by what performs and keep them current. A source's views and engagement are another
      eye's evidence too, never the person's taste. A reference found this way is still read once, as VF1 reads any
      link, and VF1's rights confirmation still applies.

    Without a pool, everything here still works locally, and clef runs on the person's own Cloudflare account.

## Deferred

- **An automatic judge for the videoform's test bench:** a video-capable language model, paid for by the person, in
  the bench only and never shipped. It would answer fidelity, pairs and gaps (what the references show that the
  videoform doesn't hold), calibrated against the person's verdicts.
- **The order of the videoform's test sets.**
- **brief.ts's references and look, the project's look file (DR18) and the video-decompiler skill's targets** stay
  where they are. Shane chose the six above on 11-20, and these may join later.
