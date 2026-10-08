MUTS = [
  ('M04-near-to-exact-off-by-one', 'src/party/forks.rs',
   'Remaining::Near(1) => Remaining::Exact(usize::MAX),',
   'Remaining::Near(1) => Remaining::Exact(usize::MAX - 1),'),
  ('M22-skip-recursive', 'src/party/io/reader.rs',
   '''    pub fn skip(&mut self) {
        let mut pending = 1u64;
        while pending != 0 {
            pending -= 1;
            if let PartyNode::Branch(branch) = self.read() {
                pending += u64::from(branch.has_left_child()) + u64::from(branch.has_right_child());
            }
        }''',
   '''    pub fn skip(&mut self) {
        if let PartyNode::Branch(branch) = self.read() {
            if branch.has_left_child() {
                self.skip();
            }
            if branch.has_right_child() {
                self.skip();
            }
        }'''),
  ('M23-overlay-shallow-depth', 'src/shape.rs',
   'depth: self.version.depth().max(self.party.depth()),',
   'depth: self.version.depth().min(self.party.depth()),'),
  ('M24-shares-ceil-floor-swapped', 'src/party/forks.rs',
   '            self.pending.push((right, count / 2));\n            self.pending.push((party, count.div_ceil(2)));',
   '            self.pending.push((right, count.div_ceil(2)));\n            self.pending.push((party, count / 2));'),
]
