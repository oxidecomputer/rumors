import sys
W=sys.argv[1]
def swap(path, old, new):
    p=W+'/'+path
    t=open(p).read()
    n=t.count(old)
    assert n==1, (path, n)
    open(p,'w').write(t.replace(old,new))
    print('swapped', path)

helper = '''
/// Surveyor scratch switch (never committed): whether the compile-time
/// `SURVEY_MUTANT` environment variable names `name`.
pub(crate) const fn survey_mutant(name: &str) -> bool {
    let actual = match option_env!("SURVEY_MUTANT") {
        Some(s) => s.as_bytes(),
        None => return false,
    };
    let wanted = name.as_bytes();
    if actual.len() != wanted.len() {
        return false;
    }
    let mut i = 0;
    while i < actual.len() {
        if actual[i] != wanted[i] {
            return false;
        }
        i += 1;
    }
    true
}
'''
p=W+'/lib.rs'; t=open(p).read(); open(p,'w').write(t+helper); print('appended lib.rs')

swap('bits/stack/packed_u64.rs',
     '        let width = if quick < 62 {',
     '        const SURVEY_PACKED: bool = crate::survey_mutant("packed");\n        let width = if (if SURVEY_PACKED { quick > 62 } else { quick < 62 }) {')
swap('bits/writer.rs',
     '        if whole > 0 {',
     '        const SURVEY_SPLICE: bool = crate::survey_mutant("splice");\n        if (if SURVEY_SPLICE { whole == 0 } else { whole > 0 }) {')
swap('bits/reader/words.rs',
     '''    fn read_word_opt(&mut self) -> Option<u32> {
        if self.next >= self.total {''',
     '''    fn read_word_opt(&mut self) -> Option<u32> {
        const SURVEY_WORDS_NONE: bool = crate::survey_mutant("words_none");
        const SURVEY_WORDS_LT: bool = crate::survey_mutant("words_lt");
        if SURVEY_WORDS_NONE {
            return None;
        }
        if (if SURVEY_WORDS_LT { self.next < self.total } else { self.next >= self.total }) {''')
