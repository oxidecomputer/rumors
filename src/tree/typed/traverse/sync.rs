pub fn sync() {}

// I think we need to hash the party and version into the path, so that it's
// impossible for the history of a value to be acausal (it tracks only actions
// by a particular party, with a particular version, which will never repeat).
// This then allows us to assume leaves will *never* collide. We should produce
// the hash by hash(hash(party) || hash(version) || hash(message)), so that
// there are no length-malleability issues. We should pick Bytes as the concrete
// type for a party, eliminating the polymorphism; it's the right choice. And
// then messages become pairs of (party, version, message), and deletion is
// *always* at-most-once for a given point in the space -- if you re-insert the
// same thing later, it acquires a different version, and hence a different
// hash. This means our version-vector split to determine the directionality of
// deletion is now sound, because if something exists on only one side,
// whichever is causally non-prior *must* dominate, because once deleted,
// something *stays* deleted, so it's safe to propagate deletion. I just
// implemented this, but now the tests are failing. Tomorrow: pick up, fix the
// tests, move onto implemeting sync.
