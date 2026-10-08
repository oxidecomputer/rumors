# Mutation survivors: party

26 survivors. Each entry: the cargo-mutants name, the enclosing function,
and the exact diff cargo-mutants applied (apply with `patch -p0` from the worktree root
after stripping the `*** ` header lines, or re-create it by hand).

## party-1: `crates/before/src/party.rs:150:9`

- name: `crates/before/src/party.rs:150:9: replace <impl core::hash::Hash for Party>::hash with ()`
- function: `<impl core::hash::Hash for Party>::hash`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/party.rs
+++ replace <impl core::hash::Hash for Party>::hash with ()
@@ -142,17 +142,17 @@
 ///
 #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/party_hash.html")))]
 #[cfg_attr(
     not(doc),
     doc = "`O(n)` in total input bytes; `O(|self|)`: one pass over the canonical bytes"
 )]
 impl core::hash::Hash for Party {
     fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
-        core::hash::Hash::hash(&self.0, state);
+        () /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl Party {
     /// Adopt a canonical [`Party`] representation produced or validated by its
     /// I/O boundary.
     pub(crate) fn from_canonical(bits: Bits) -> Self {
         Party(bits)
```

## party-2: `crates/before/src/party/compare.rs:149:30`

- name: `crates/before/src/party/compare.rs:149:30: replace || with && in Comparison<'a, 'b>::descend`
- function: `Comparison<'a, 'b>::descend`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/party/compare.rs
+++ replace || with && in Comparison<'a, 'b>::descend
@@ -141,17 +141,17 @@
         let first_left = first.has_left_child();
         let first_right = first.has_right_child();
         let second_left = second.has_left_child();
         let second_right = second.has_right_child();
         if (first_left || second_left) && (first_right || second_right) {
             self.pending.push(first_right);
             self.pending.push(second_right);
             (self.first_present, self.second_present) = (first_left, second_left);
-        } else if first_left || second_left {
+        } else if first_left && /* ~ changed by cargo-mutants ~ */ second_left {
             (self.first_present, self.second_present) = (first_left, second_left);
         } else {
             (self.first_present, self.second_present) = (first_right, second_right);
         }
     }
 
     /// Complete the current pair with a passing verdict: step into the
     /// innermost queued right pair (`Continue`), or report the whole walk done
```

## party-3: `crates/before/src/party/fork.rs:101:60`

- name: `crates/before/src/party/fork.rs:101:60: replace + with * in Party::fork_tree`
- function: `Party::fork_tree`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/party/fork.rs
+++ replace + with * in Party::fork_tree
@@ -93,17 +93,17 @@
     /// complete input tree. Its right child's end is therefore the end of the
     /// input.
     pub(crate) fn fork_tree(mut source: PartyReader<'_>) -> (Party, Party) {
         match source.next_fork() {
             ForkPoint::Owned(common_path) => {
                 // The shared unary path and the newly divided terminal have
                 // known exact sizes. Sizing each result here prevents a small
                 // half from retaining storage proportional to the source.
-                let output_bits = common_path.stored_len() + 4;
+                let output_bits = common_path.stored_len() * /* ~ changed by cargo-mutants ~ */ 4;
                 let mut keep = PartyWriter::with_capacity(output_bits);
                 let mut give = PartyWriter::with_capacity(output_bits);
                 keep.copy_shared_unary_path(common_path, &mut give);
                 // Splitting a fully owned region gives one half to each
                 // result.
                 keep.branch(PartyBranch::Left);
                 keep.owned();
                 give.branch(PartyBranch::Right);
```

## party-4: `crates/before/src/party/fork.rs:119:60`

- name: `crates/before/src/party/fork.rs:119:60: replace + with * in Party::fork_tree`
- function: `Party::fork_tree`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/party/fork.rs
+++ replace + with * in Party::fork_tree
@@ -111,17 +111,17 @@
                 (keep.finish(), give.finish())
             }
             ForkPoint::Children(common_path) => {
                 // Delimit the left child once. The right child is the source
                 // suffix, so both output sizes are then known without another
                 // traversal.
                 let (_, left) = source.take_subtree();
                 let right = source.remainder();
-                let prefix_bits = common_path.stored_len() + 2;
+                let prefix_bits = common_path.stored_len() * /* ~ changed by cargo-mutants ~ */ 2;
                 let mut keep = PartyWriter::with_capacity(prefix_bits + left.stored_len());
                 let mut give = PartyWriter::with_capacity(prefix_bits + right.stored_len());
                 keep.copy_shared_unary_path(common_path, &mut give);
                 // Each child is already one complete half of the union.
                 keep.branch(PartyBranch::Left);
                 keep.copy_subtree(left);
                 give.branch(PartyBranch::Right);
                 give.copy_subtree(right);
```

## party-5: `crates/before/src/party/forks.rs:61:47`

- name: `crates/before/src/party/forks.rs:61:47: replace && with || in SharePath<'_>::next_decision`
- function: `SharePath<'_>::next_decision`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/party/forks.rs
+++ replace && with || in SharePath<'_>::next_decision
@@ -53,17 +53,17 @@
     /// Take the next direction from the balanced plan.
     ///
     /// The `depth` bits of `index` name the base share from root to leaf. A base
     /// share selected for one extra fork appends `second` as its final
     /// decision.
     fn next_decision(&mut self) -> Option<bool> {
         let direction = if self.decision < self.depth {
             self.index.bit(self.depth - 1 - self.decision)
-        } else if self.decision == self.depth && self.forks_again {
+        } else if self.decision == self.depth || /* ~ changed by cargo-mutants ~ */ self.forks_again {
             self.second
         } else {
             return None;
         };
         self.decision += 1;
         Some(direction)
     }
 }
```

## party-6: `crates/before/src/party/forks.rs:139:63`

- name: `crates/before/src/party/forks.rs:139:63: replace - with + in Remaining::advance`
- function: `Remaining::advance`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/party/forks.rs
+++ replace - with + in Remaining::advance
@@ -131,17 +131,17 @@
         usize::try_from(excess).map_or(Remaining::Distant, Remaining::Near)
     }
 
     /// Account for one produced share.
     fn advance(&mut self) {
         *self = match *self {
             Remaining::Exact(remaining) => Remaining::Exact(remaining - 1),
             Remaining::Near(1) => Remaining::Exact(usize::MAX),
-            Remaining::Near(excess) => Remaining::Near(excess - 1),
+            Remaining::Near(excess) => Remaining::Near(excess + /* ~ changed by cargo-mutants ~ */ 1),
             Remaining::Distant => Remaining::Distant,
         };
     }
 
     /// Whether an exactly tracked plan is exhausted.
     fn is_empty(&self) -> bool {
         matches!(self, Remaining::Exact(0))
     }
```

## party-7: `crates/before/src/party/forks.rs:139:63`

- name: `crates/before/src/party/forks.rs:139:63: replace - with / in Remaining::advance`
- function: `Remaining::advance`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/party/forks.rs
+++ replace - with / in Remaining::advance
@@ -131,17 +131,17 @@
         usize::try_from(excess).map_or(Remaining::Distant, Remaining::Near)
     }
 
     /// Account for one produced share.
     fn advance(&mut self) {
         *self = match *self {
             Remaining::Exact(remaining) => Remaining::Exact(remaining - 1),
             Remaining::Near(1) => Remaining::Exact(usize::MAX),
-            Remaining::Near(excess) => Remaining::Near(excess - 1),
+            Remaining::Near(excess) => Remaining::Near(excess / /* ~ changed by cargo-mutants ~ */ 1),
             Remaining::Distant => Remaining::Distant,
         };
     }
 
     /// Whether an exactly tracked plan is exhausted.
     fn is_empty(&self) -> bool {
         matches!(self, Remaining::Exact(0))
     }
```

## party-8: `crates/before/src/party/io.rs:29:26`

- name: `crates/before/src/party/io.rs:29:26: replace > with >= in Party::decode_prefix`
- function: `Party::decode_prefix`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/party/io.rs
+++ replace > with >= in Party::decode_prefix
@@ -21,17 +21,17 @@
 
     /// Validate and adopt the first byte-aligned Party in `bytes`.
     ///
     /// The returned byte count starts the next encoded field. The Party shares
     /// its allocation with `bytes`.
     pub(crate) fn decode_prefix(bytes: &bytes::Bytes) -> Result<(Self, usize), Decode> {
         let end = validate::prefix(BitsReader::from_bytes(bytes))?;
         let encoded_bytes = (end + 1).div_ceil(8);
-        if encoded_bytes > bytes.len() as u64 {
+        if encoded_bytes >= /* ~ changed by cargo-mutants ~ */ bytes.len() as u64 {
             return Err(Decode::Truncated);
         }
         let encoded_bytes =
             usize::try_from(encoded_bytes).expect("the Party prefix ends within the input buffer");
         Bits::validate_padding(&bytes[..encoded_bytes], end)?;
         Ok((
             Self::from_canonical(Bits::from_canonical(bytes.slice(..encoded_bytes))),
             encoded_bytes,
```

## party-9: `crates/before/src/party/io/reader.rs:48:9`

- name: `crates/before/src/party/io/reader.rs:48:9: replace PartyPath<'a>::stored_len -> u64 with 0`
- function: `PartyPath<'a>::stored_len`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/party/io/reader.rs
+++ replace PartyPath<'a>::stored_len -> u64 with 0
@@ -40,17 +40,17 @@
     start: u64,
     /// First bit after the final branch tag.
     end: u64,
 }
 
 impl<'a> PartyPath<'a> {
     /// Number of stored bits in the shared path.
     pub fn stored_len(&self) -> u64 {
-        self.end - self.start
+        0 /* ~ changed by cargo-mutants ~ */
     }
 
     /// Expose the delimited storage only to the Party writer.
     pub fn storage(self) -> (&'a Bits, u64, u64) {
         (self.bits, self.start, self.end)
     }
 }
```

## party-10: `crates/before/src/party/io/reader.rs:48:9`

- name: `crates/before/src/party/io/reader.rs:48:9: replace PartyPath<'a>::stored_len -> u64 with 1`
- function: `PartyPath<'a>::stored_len`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/party/io/reader.rs
+++ replace PartyPath<'a>::stored_len -> u64 with 1
@@ -40,17 +40,17 @@
     start: u64,
     /// First bit after the final branch tag.
     end: u64,
 }
 
 impl<'a> PartyPath<'a> {
     /// Number of stored bits in the shared path.
     pub fn stored_len(&self) -> u64 {
-        self.end - self.start
+        1 /* ~ changed by cargo-mutants ~ */
     }
 
     /// Expose the delimited storage only to the Party writer.
     pub fn storage(self) -> (&'a Bits, u64, u64) {
         (self.bits, self.start, self.end)
     }
 }
```

## party-11: `crates/before/src/party/io/reader.rs:48:18`

- name: `crates/before/src/party/io/reader.rs:48:18: replace - with + in PartyPath<'a>::stored_len`
- function: `PartyPath<'a>::stored_len`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/party/io/reader.rs
+++ replace - with + in PartyPath<'a>::stored_len
@@ -40,17 +40,17 @@
     start: u64,
     /// First bit after the final branch tag.
     end: u64,
 }
 
 impl<'a> PartyPath<'a> {
     /// Number of stored bits in the shared path.
     pub fn stored_len(&self) -> u64 {
-        self.end - self.start
+        self.end + /* ~ changed by cargo-mutants ~ */ self.start
     }
 
     /// Expose the delimited storage only to the Party writer.
     pub fn storage(self) -> (&'a Bits, u64, u64) {
         (self.bits, self.start, self.end)
     }
 }
```

## party-12: `crates/before/src/party/io/reader.rs:73:9`

- name: `crates/before/src/party/io/reader.rs:73:9: replace PartySubtree<'a>::stored_len -> u64 with 0`
- function: `PartySubtree<'a>::stored_len`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/party/io/reader.rs
+++ replace PartySubtree<'a>::stored_len -> u64 with 0
@@ -65,17 +65,17 @@
 impl<'a> PartySubtree<'a> {
     /// Open a reader at the subtree root.
     pub fn reader(self) -> PartyReader<'a> {
         PartyReader::from_storage(self.bits, self.start, self.end)
     }
 
     /// Number of stored bits in the subtree.
     pub fn stored_len(&self) -> u64 {
-        self.end - self.start
+        0 /* ~ changed by cargo-mutants ~ */
     }
 
     /// Expose the delimited storage only to the Party writer.
     pub fn storage(self) -> (&'a Bits, u64, u64) {
         (self.bits, self.start, self.end)
     }
 
     /// Whether the subtree root is a branch.
```

## party-13: `crates/before/src/party/io/reader.rs:73:9`

- name: `crates/before/src/party/io/reader.rs:73:9: replace PartySubtree<'a>::stored_len -> u64 with 1`
- function: `PartySubtree<'a>::stored_len`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/party/io/reader.rs
+++ replace PartySubtree<'a>::stored_len -> u64 with 1
@@ -65,17 +65,17 @@
 impl<'a> PartySubtree<'a> {
     /// Open a reader at the subtree root.
     pub fn reader(self) -> PartyReader<'a> {
         PartyReader::from_storage(self.bits, self.start, self.end)
     }
 
     /// Number of stored bits in the subtree.
     pub fn stored_len(&self) -> u64 {
-        self.end - self.start
+        1 /* ~ changed by cargo-mutants ~ */
     }
 
     /// Expose the delimited storage only to the Party writer.
     pub fn storage(self) -> (&'a Bits, u64, u64) {
         (self.bits, self.start, self.end)
     }
 
     /// Whether the subtree root is a branch.
```

## party-14: `crates/before/src/party/io/reader.rs:73:18`

- name: `crates/before/src/party/io/reader.rs:73:18: replace - with / in PartySubtree<'a>::stored_len`
- function: `PartySubtree<'a>::stored_len`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/party/io/reader.rs
+++ replace - with / in PartySubtree<'a>::stored_len
@@ -65,17 +65,17 @@
 impl<'a> PartySubtree<'a> {
     /// Open a reader at the subtree root.
     pub fn reader(self) -> PartyReader<'a> {
         PartyReader::from_storage(self.bits, self.start, self.end)
     }
 
     /// Number of stored bits in the subtree.
     pub fn stored_len(&self) -> u64 {
-        self.end - self.start
+        self.end / /* ~ changed by cargo-mutants ~ */ self.start
     }
 
     /// Expose the delimited storage only to the Party writer.
     pub fn storage(self) -> (&'a Bits, u64, u64) {
         (self.bits, self.start, self.end)
     }
 
     /// Whether the subtree root is a branch.
```

## party-15: `crates/before/src/party/io/reader.rs:83:9`

- name: `crates/before/src/party/io/reader.rs:83:9: replace PartySubtree<'a>::is_branch -> bool with true`
- function: `PartySubtree<'a>::is_branch`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/party/io/reader.rs
+++ replace PartySubtree<'a>::is_branch -> bool with true
@@ -75,20 +75,17 @@
 
     /// Expose the delimited storage only to the Party writer.
     pub fn storage(self) -> (&'a Bits, u64, u64) {
         (self.bits, self.start, self.end)
     }
 
     /// Whether the subtree root is a branch.
     pub fn is_branch(&self) -> bool {
-        !matches!(
-            PartyReader::node_at(self.bits, self.start),
-            PartyNode::Owned
-        )
+        true /* ~ changed by cargo-mutants ~ */
     }
 }
 
 /// An owned read-only snapshot of a party tree.
 ///
 /// Fork planning needs to revisit the original shape after the live party has
 /// been narrowed. This type makes that retained topology explicit without
 /// pretending it is another live, linearly owned [`Party`].
```

## party-16: `crates/before/src/party/io/reader.rs:220:9`

- name: `crates/before/src/party/io/reader.rs:220:9: replace PartyReader<'a>::source_len -> u64 with 0`
- function: `PartyReader<'a>::source_len`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/party/io/reader.rs
+++ replace PartyReader<'a>::source_len -> u64 with 0
@@ -212,17 +212,17 @@
     /// Start at a live party's root.
     pub fn for_party(party: &'a Party) -> Self {
         let end = Self::source_len_of(&party.0);
         Self::from_storage(&party.0, 0, end)
     }
 
     /// The length of the canonical Party stream backing this reader.
     pub fn source_len(&self) -> u64 {
-        Self::source_len_of(self.bits)
+        0 /* ~ changed by cargo-mutants ~ */
     }
 
     /// Start at validated storage. Tests use this to exercise internal walks on
     /// constructed canonical trees without creating a live party.
     #[cfg(test)]
     pub fn from_bits(bits: &'a Bits) -> Self {
         let end = Self::source_len_of(bits);
         Self::from_storage(bits, 0, end)
```

## party-17: `crates/before/src/party/io/reader.rs:220:9`

- name: `crates/before/src/party/io/reader.rs:220:9: replace PartyReader<'a>::source_len -> u64 with 1`
- function: `PartyReader<'a>::source_len`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/party/io/reader.rs
+++ replace PartyReader<'a>::source_len -> u64 with 1
@@ -212,17 +212,17 @@
     /// Start at a live party's root.
     pub fn for_party(party: &'a Party) -> Self {
         let end = Self::source_len_of(&party.0);
         Self::from_storage(&party.0, 0, end)
     }
 
     /// The length of the canonical Party stream backing this reader.
     pub fn source_len(&self) -> u64 {
-        Self::source_len_of(self.bits)
+        1 /* ~ changed by cargo-mutants ~ */
     }
 
     /// Start at validated storage. Tests use this to exercise internal walks on
     /// constructed canonical trees without creating a live party.
     #[cfg(test)]
     pub fn from_bits(bits: &'a Bits) -> Self {
         let end = Self::source_len_of(bits);
         Self::from_storage(bits, 0, end)
```

## party-18: `crates/before/src/party/io/reader.rs:331:9`

- name: `crates/before/src/party/io/reader.rs:331:9: replace PartyReader<'a>::stored_len -> u64 with 0`
- function: `PartyReader<'a>::stored_len`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/party/io/reader.rs
+++ replace PartyReader<'a>::stored_len -> u64 with 0
@@ -323,17 +323,17 @@
 
     /// Restart at a known subtree in the same party.
     pub fn restart_at(&self, pos: u64) -> PartyReader<'a> {
         Self::from_storage(self.bits, pos, self.end)
     }
 
     /// The unread subtree's stored bit length, for builder sizing only.
     pub fn stored_len(&self) -> u64 {
-        self.end - self.position
+        0 /* ~ changed by cargo-mutants ~ */
     }
 
     /// The current bit offset.
     pub fn offset(&self) -> u64 {
         self.position
     }
 
     /// Whether the reader has consumed its containing tree.
```

## party-19: `crates/before/src/party/io/reader.rs:331:9`

- name: `crates/before/src/party/io/reader.rs:331:9: replace PartyReader<'a>::stored_len -> u64 with 1`
- function: `PartyReader<'a>::stored_len`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/party/io/reader.rs
+++ replace PartyReader<'a>::stored_len -> u64 with 1
@@ -323,17 +323,17 @@
 
     /// Restart at a known subtree in the same party.
     pub fn restart_at(&self, pos: u64) -> PartyReader<'a> {
         Self::from_storage(self.bits, pos, self.end)
     }
 
     /// The unread subtree's stored bit length, for builder sizing only.
     pub fn stored_len(&self) -> u64 {
-        self.end - self.position
+        1 /* ~ changed by cargo-mutants ~ */
     }
 
     /// The current bit offset.
     pub fn offset(&self) -> u64 {
         self.position
     }
 
     /// Whether the reader has consumed its containing tree.
```

## party-20: `crates/before/src/party/io/reader.rs:331:18`

- name: `crates/before/src/party/io/reader.rs:331:18: replace - with + in PartyReader<'a>::stored_len`
- function: `PartyReader<'a>::stored_len`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/party/io/reader.rs
+++ replace - with + in PartyReader<'a>::stored_len
@@ -323,17 +323,17 @@
 
     /// Restart at a known subtree in the same party.
     pub fn restart_at(&self, pos: u64) -> PartyReader<'a> {
         Self::from_storage(self.bits, pos, self.end)
     }
 
     /// The unread subtree's stored bit length, for builder sizing only.
     pub fn stored_len(&self) -> u64 {
-        self.end - self.position
+        self.end + /* ~ changed by cargo-mutants ~ */ self.position
     }
 
     /// The current bit offset.
     pub fn offset(&self) -> u64 {
         self.position
     }
 
     /// Whether the reader has consumed its containing tree.
```

## party-21: `crates/before/src/party/io/reader.rs:341:9`

- name: `crates/before/src/party/io/reader.rs:341:9: replace PartyReader<'a>::at_end -> bool with true`
- function: `PartyReader<'a>::at_end`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/party/io/reader.rs
+++ replace PartyReader<'a>::at_end -> bool with true
@@ -333,17 +333,17 @@
 
     /// The current bit offset.
     pub fn offset(&self) -> u64 {
         self.position
     }
 
     /// Whether the reader has consumed its containing tree.
     pub fn at_end(&self) -> bool {
-        self.position == self.end
+        true /* ~ changed by cargo-mutants ~ */
     }
 
     /// Consume and delimit the complete current subtree.
     pub fn take_subtree(&mut self) -> (PartyNode, PartySubtree<'a>) {
         let start = self.position;
         let node = self.read();
         self.skip_present_children(node);
         (
```

## party-22: `crates/before/src/party/io/writer/positions.rs:46:21`

- name: `crates/before/src/party/io/writer/positions.rs:46:21: replace > with < in Positions::push`
- function: `Positions::push`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/party/io/writer/positions.rs
+++ replace > with < in Positions::push
@@ -38,17 +38,17 @@
             values: PackedU64Stack::new(),
         }
     }
 
     /// Retain a newly reserved tag position.
     pub fn push(&mut self, OpenBranch(pos): OpenBranch) {
         debug_assert!(pos >= self.top, "reserved tag positions never move left");
         let distance = pos - self.top;
-        if self.len > 0 && distance == TAG_BITS as u64 {
+        if self.len < /* ~ changed by cargo-mutants ~ */ 0 && distance == TAG_BITS as u64 {
             self.adjacent += 1;
         } else {
             self.flush_run();
             self.records.push(false);
             // The packed stack stores positive integers; the first position may be 0.
             self.values.push(distance + 1);
         }
         self.top = pos;
```

## party-23: `crates/before/src/party/io/writer/positions.rs:46:21`

- name: `crates/before/src/party/io/writer/positions.rs:46:21: replace > with == in Positions::push`
- function: `Positions::push`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/party/io/writer/positions.rs
+++ replace > with == in Positions::push
@@ -38,17 +38,17 @@
             values: PackedU64Stack::new(),
         }
     }
 
     /// Retain a newly reserved tag position.
     pub fn push(&mut self, OpenBranch(pos): OpenBranch) {
         debug_assert!(pos >= self.top, "reserved tag positions never move left");
         let distance = pos - self.top;
-        if self.len > 0 && distance == TAG_BITS as u64 {
+        if self.len == /* ~ changed by cargo-mutants ~ */ 0 && distance == TAG_BITS as u64 {
             self.adjacent += 1;
         } else {
             self.flush_run();
             self.records.push(false);
             // The packed stack stores positive integers; the first position may be 0.
             self.values.push(distance + 1);
         }
         self.top = pos;
```

## party-24: `crates/before/src/party/io/writer/positions.rs:46:21`

- name: `crates/before/src/party/io/writer/positions.rs:46:21: replace > with >= in Positions::push`
- function: `Positions::push`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/party/io/writer/positions.rs
+++ replace > with >= in Positions::push
@@ -38,17 +38,17 @@
             values: PackedU64Stack::new(),
         }
     }
 
     /// Retain a newly reserved tag position.
     pub fn push(&mut self, OpenBranch(pos): OpenBranch) {
         debug_assert!(pos >= self.top, "reserved tag positions never move left");
         let distance = pos - self.top;
-        if self.len > 0 && distance == TAG_BITS as u64 {
+        if self.len >= /* ~ changed by cargo-mutants ~ */ 0 && distance == TAG_BITS as u64 {
             self.adjacent += 1;
         } else {
             self.flush_run();
             self.records.push(false);
             // The packed stack stores positive integers; the first position may be 0.
             self.values.push(distance + 1);
         }
         self.top = pos;
```

## party-25: `crates/before/src/party/io/writer/positions.rs:62:18`

- name: `crates/before/src/party/io/writer/positions.rs:62:18: replace -= with += in Positions::pop`
- function: `Positions::pop`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/party/io/writer/positions.rs
+++ replace -= with += in Positions::pop
@@ -54,17 +54,17 @@
         self.top = pos;
         self.len += 1;
     }
 
     /// Restore the newest tag position.
     pub fn pop(&mut self) -> OpenBranch {
         assert!(self.len > 0, "position stack underflow");
         let pos = self.top;
-        self.len -= 1;
+        self.len += /* ~ changed by cargo-mutants ~ */ 1;
         if self.adjacent > 0 {
             self.adjacent -= 1;
             self.top -= TAG_BITS as u64;
             return OpenBranch(pos);
         }
 
         let is_run = self.records.pop().expect("position stack underflow");
         let value = self.values.pop();
```

## party-26: `crates/before/src/party/io/writer/positions.rs:62:18`

- name: `crates/before/src/party/io/writer/positions.rs:62:18: replace -= with /= in Positions::pop`
- function: `Positions::pop`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/party/io/writer/positions.rs
+++ replace -= with /= in Positions::pop
@@ -54,17 +54,17 @@
         self.top = pos;
         self.len += 1;
     }
 
     /// Restore the newest tag position.
     pub fn pop(&mut self) -> OpenBranch {
         assert!(self.len > 0, "position stack underflow");
         let pos = self.top;
-        self.len -= 1;
+        self.len /= /* ~ changed by cargo-mutants ~ */ 1;
         if self.adjacent > 0 {
             self.adjacent -= 1;
             self.top -= TAG_BITS as u64;
             return OpenBranch(pos);
         }
 
         let is_run = self.records.pop().expect("position stack underflow");
         let value = self.values.pop();
```

