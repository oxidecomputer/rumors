//! `From<Clock>` for `[Clock; N]` requires `N >= 1`, for the same reason as the
//! `Party` split: a clock owns a nonempty party and cannot vanish into zero
//! shares. `[Clock; 0]::from` must fail to compile.

use before::Clock;

fn main() {
    let _empty: [Clock; 0] = Clock::seed().into();
}
