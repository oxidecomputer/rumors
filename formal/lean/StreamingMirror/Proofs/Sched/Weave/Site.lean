/-
Window-site assembly (PROGRESS.md §7 3b, layer D): the per-site
wrappers that turn the counting layer's pins into the window lemmas'
hypothesis packages. The `hsnd` family here is the first tier: each
window site's emitted count equals the seq of the event being
emitted, read as total minus the site's collapsed `futLen` share
(`Emit.lean`'s site pins), with the channel bridged from the pins'
`wpk` spelling to the windows' party-indexed spelling.
-/
import StreamingMirror.Proofs.Sched.Weave.Emit

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ================================================= the hsnd wrappers

/-- The summary site's `hsnd`: the walk has emitted exactly the
summaries of the scopes before its current one. -/
theorem upper_site_hsnd (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {p : Party} {hh k : Nat}
    (hna : asks p hh = false) (hhr : hh < sk.rootH)
    (hk : k < sk.stageLen hh)
    (hfu : futLen sk fut (walkIdx sk hh) (upperOut (wpk hh)) true
      = sk.stageLen hh - k) :
    sndCount (Chan.upper p hh) st.out = k := by
  have hch : upperOut (wpk hh) = Chan.upper p hh := by
    rw [show upperOut (wpk hh) = Chan.upper (wpk hh).1 hh from rfl,
      wpk_fst_of_answerer hna]
  have hpin := upper_snd_pin sk hwf h hhr
  rw [hch] at hpin hfu
  omega

/-- The resolution site's `hsnd`: the walk has emitted exactly the
resolutions before its current slot's. -/
theorem lower_site_hsnd (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {p : Party} {hh k i : Nat}
    (hna : asks p hh = false) (hhr : hh < sk.rootH)
    (hfu : futLen sk fut (walkIdx sk hh) (lowerOut (wpk hh)) true
      = sk.dsBefore hh (sk.stageLen hh)
        - (sk.dsBefore hh k + dRank sk (wpk hh) k i))
    (hbnd : sk.dsBefore hh k + dRank sk (wpk hh) k i
      < sk.dsBefore hh (sk.stageLen hh)) :
    sndCount (Chan.lower p hh) st.out
      = sk.dsBefore hh k + dRank sk (wpk hh) k i := by
  have hch : lowerOut (wpk hh) = Chan.lower p hh := by
    rw [show lowerOut (wpk hh) = Chan.lower (wpk hh).1 hh from rfl,
      wpk_fst_of_answerer hna]
  have hpin := lower_snd_pin sk hwf h hhr
  rw [hch] at hpin hfu
  omega

/-- The leaf-wire site's `hsnd`: the stage-0 walk has emitted exactly
the wires before its current slot's. -/
theorem wire0_site_hsnd (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {k i : Nat}
    (hr : 0 < sk.rootH)
    (hfu : futLen sk fut (walkIdx sk 0) (wireOut (wpk 0)) true
      = sk.wiresBefore 0 (sk.stageLen 0) - (sk.wiresBefore 0 k + i))
    (hbnd : sk.wiresBefore 0 k + i
      < sk.wiresBefore 0 (sk.stageLen 0)) :
    sndCount (Chan.wire Party.R 0) st.out = sk.wiresBefore 0 k + i := by
  have hch : wireOut (wpk 0) = Chan.wire Party.R 0 := rfl
  have hpin := wire_snd_pin sk hwf h hr
  rw [hch] at hpin hfu
  omega

/-- The leaf-request site's `hsnd`: the stage-1 walk has emitted
exactly the requests before its current feed cursor's. -/
theorem leafreq_site_hsnd (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {K i t : Nat}
    (hr : 1 < sk.rootH)
    (hfu : futLen sk fut (walkIdx sk 1) (askedOut (wpk 1)) true
      = sk.qsBefore 1 (sk.stageLen 1)
        - (sk.qsBefore 1 K + qSum sk (wpk 1) K i + t))
    (hbnd : sk.qsBefore 1 K + qSum sk (wpk 1) K i + t
      < sk.qsBefore 1 (sk.stageLen 1)) :
    sndCount Chan.leafRequests st.out
      = sk.qsBefore 1 K + qSum sk (wpk 1) K i + t := by
  have hch : askedOut (wpk 1) = Chan.leafRequests := rfl
  have hpin := asked_snd_pin sk hwf h (Nat.le_refl 1) hr
  rw [hch] at hpin hfu
  omega

end StreamingMirror.Sched
