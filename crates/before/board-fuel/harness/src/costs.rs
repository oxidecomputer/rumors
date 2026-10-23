//! The fuel price of every wasm operator, stated here rather than inherited
//! from Wasmtime's defaults.
//!
//! Fuel is only as stable as its price list. Wasmtime 47 charged nothing per
//! byte for `memory.copy` and `memory.fill`, and Wasmtime 48 charges one, so
//! the same guest reads differently under each, and a quadratic run of
//! element moves inside a `Vec` is visible under only one of them. The
//! harness therefore builds the whole table itself: a flat cost for every
//! operator in Wasmtime's own operator list, and every operand-proportional
//! cost by name. A Wasmtime release that adds a variable cost fails to compile
//! here until the new cost is priced. One that adds an operator compiles, and
//! the new operator is priced 1 like any other; the operator count that
//! [`render_costs`] records in the calibration's provenance then changes, so
//! the run refuses to judge until the ceilings are recalibrated.
//!
//! [`render_costs`] summarizes a table for the calibration's provenance, so
//! ceilings calibrated under one table are refused under another.

use std::collections::BTreeMap;

use wasmtime::wasmparser::for_each_operator;
use wasmtime::{OperatorCost, VariableOperatorCost};

/// The operators priced at zero: those that generate no machine code of
/// their own.
///
/// `Nop` and `Drop` compile to nothing, and the structured control markers
/// (`Block`, `Loop`, `Else`, `End`) and the function-ending `Return` and
/// `Unreachable` delimit code rather than execute work. Every other operator
/// costs one. This is the schedule Wasmtime 48 itself applies by default,
/// restated so that it is pinned here.
const FREE_OPERATORS: [&str; 8] = [
    "Block",
    "Drop",
    "Else",
    "End",
    "Loop",
    "Nop",
    "Return",
    "Unreachable",
];

/// The flat cost of the operator named `operator`.
fn flat_cost(operator: &str) -> u8 {
    u8::from(!FREE_OPERATORS.contains(&operator))
}

/// The costs proportional to an operator's runtime operand.
///
/// Every bulk copy, fill, and initialization costs one per byte or element
/// moved, so a `memmove` of `n` bytes, such as a shift inside a `Vec`, costs
/// `n`. Growing memory costs nothing per page: the board measures memory as
/// heap, and fuel prices executed work.
const VARIABLE_COSTS: VariableOperatorCost = VariableOperatorCost {
    memory_copy_per_byte: 1,
    memory_fill_per_byte: 1,
    memory_init_per_byte: 1,
    memory_grow_per_page: 0,
    table_copy_per_element: 1,
    table_fill_per_element: 1,
    table_init_per_element: 1,
    table_grow_per_element: 1,
    array_copy_per_element: 1,
    array_fill_per_element: 1,
    array_new_data_per_element: 1,
    array_init_data_per_element: 1,
    array_new_elem_per_element: 1,
    array_init_elem_per_element: 1,
    array_new_default_per_element: 1,
    array_new_per_element: 1,
};

/// Defines [`operator_cost`] and [`flat_costs`] over Wasmtime's operator list,
/// the list its [`OperatorCost`] has one field for each entry of.
macro_rules! define_costs {
    ($(
        @$proposal:ident $op:ident $({ $($arg:ident: $argty:ty),* })?
            => $visit:ident ($($ann:tt)*)
    )*) => {
        /// Returns the cost table every fuel engine charges.
        pub fn operator_cost() -> OperatorCost {
            OperatorCost {
                $( $op: flat_cost(stringify!($op)), )*
                variable: VARIABLE_COSTS,
            }
        }

        /// Returns every operator's name and flat cost in `cost`.
        fn flat_costs(cost: &OperatorCost) -> Vec<(&'static str, u8)> {
            vec![$( (stringify!($op), cost.$op), )*]
        }
    };
}

for_each_operator!(define_costs);

/// Summarizes `cost` in one line: each flat cost with the operators it
/// prices (the most common cost as a count), then the variable costs.
///
/// Two tables render alike exactly when they price every operator alike.
pub fn render_costs(cost: &OperatorCost) -> String {
    let mut by_cost: BTreeMap<u8, Vec<&str>> = BTreeMap::new();
    for (operator, price) in flat_costs(cost) {
        by_cost.entry(price).or_default().push(operator);
    }
    let commonest = by_cost
        .iter()
        .max_by_key(|(_, operators)| operators.len())
        .map(|(&price, _)| price);
    let flat: Vec<String> = by_cost
        .iter()
        .map(|(&price, operators)| {
            if Some(price) == commonest {
                format!("{price} for the other {} operators", operators.len())
            } else {
                format!("{price} for {}", operators.join(" "))
            }
        })
        .collect();
    format!("flat {}; {:?}", flat.join("; "), cost.variable)
}
