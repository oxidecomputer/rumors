MUTS = [
  ('M19-party-join-all-duplicates-rejected-group', 'src/party.rs',
   '            if let Err(back) = self.join(group) {\n                let mut uncombined = vec![back];',
   '            if let Err(back) = self.join(group) {\n                let mut uncombined = vec![back.dangerously_alias(), back];'),
  ('M20-clock-join-all-duplicates-rejected-group', 'src/clock.rs',
   '            if let Err(back) = self.join(group) {\n                let mut uncombined = vec![back];',
   '            if let Err(back) = self.join(group) {\n                let mut uncombined = vec![back.dangerously_alias(), back];'),
  ('M21-clock-join-all-drops-groups', 'src/clock.rs',
   '                let mut uncombined = vec![back];\n                uncombined.extend(groups);\n                return Err(uncombined);',
   '                let uncombined = vec![back];\n                drop(groups);\n                return Err(uncombined);'),
]
