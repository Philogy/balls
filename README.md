# Bytecode Assembler \w Low-Level Scheduling (BALLS)

This repo is meant to be a proof of concept, investigating whether it's feasible to use exhaustive
searching for optimal stack scheduling for EVM programs.

Stack scheduling is the process of turning a sequence of variable assignments into stack
manipulating instructions. Unlike traditional, register-based ISAs, the EVMs stack based nature
doesn't map very well to assignments.

## Guide

**Installation**
1. Clone repo (`git clone https://github.com/Philogy/balls.git`)
2. `cd balls`
3. `cargo install --path .`

**Run**

`balls -h`

## Example

**ERC20 `transfer(address to, uint256 amount)` method**


Code:

```yul
// EXTERNAL
extern _REQUIRE_NOT() stack(1, 0) reads(CONTROL_FLOW)

// Define actual code
fn TRANSFER<z0>(error) -> () {
    // Define some variables
    to = calldataload(0x04)
    amount = calldataload(0x24)

    // Get from balance.
    from_bal = sload(caller())

    // Check from balance and error.
    insufficient_bal = gt(amount, from_bal)
    error' = or(insufficient_bal, error)
    _REQUIRE_NOT(error')

    // Update from balance.
    new_from_bal = sub(from_bal, amount)
    sstore(caller(), new_from_bal)

    // Update to balance.
    to_bal = sload(to)
    new_to_bal = add(to_bal, amount)
    sstore(to, new_to_bal)

    // Return success (1).
    mstore(z0, 1)
    return(z0, msize())
}
```

Compile with `balls ./examples/transfer_ma.balls -d` (`-d` tells BALLS to use the Dijkstra which is
guaranteed to result in the optimal scheduling given the constraints).

Result:


```huff
#define macro TRANSFER(z0) = takes(1) returns(0) {
    // takes:                      [error]
    caller                      // [error, caller()]
    sload                       // [error, from_bal]
    0x24                        // [error, from_bal, 0x24]
    calldataload                // [error, from_bal, amount]
    dup1                        // [error, from_bal, amount, amount]
    dup3                        // [error, from_bal, amount, amount, from_bal]
    sub                         // [error, from_bal, amount, new_from_bal]
    caller                      // [error, from_bal, amount, new_from_bal, caller()]
    sstore                      // [error, from_bal, amount]
    0x4                         // [error, from_bal, amount, 0x4]
    calldataload                // [error, from_bal, amount, to]
    dup1                        // [error, from_bal, amount, to, to]
    sload                       // [error, from_bal, amount, to, to_bal]
    dup3                        // [error, from_bal, amount, to, to_bal, amount]
    add                         // [error, from_bal, amount, to, new_to_bal]
    swap1                       // [error, from_bal, amount, new_to_bal, to]
    sstore                      // [error, from_bal, amount]
    gt                          // [error, insufficient_bal]
    or                          // [error']
    _REQUIRE_NOT()              // []
    0x1                         // [0x1]
    <z0>                        // [0x1, z0]
    mstore                      // []
    msize                       // [msize()]
    <z0>                        // [msize(), z0]
    return                      // []
    // returns:                    []
}
```

What this code does on a high-level:
- Expects an external huff macro `_REQUIRE_NOT` that takes 0 inlined arguments, consumes 1 stack
    value and pushes 0 values, that's dependent on the `CONTROL_FLOW` dependency meaning that it
    shouldn't be rearranged after e.g. a `stop`, `revert` or `return`
- Defines a Huff macro `TRANSFER` that takes 1 inlined argument `z0` and pops 1 stack value
  referencing it as `"error"`

## Extra Tips

**Default**

By default BALLS will run using the "Guessoor" scheduling algorithm, it runs quite quickly even on
unconstrained schedules but _is not guaranteed to result in the optimal scheduling_. To tune the
likelihood the result approaches the optimal scheduling you can play around with the `--guess`
parameter. Lower will make the scheduler run slower but be more likely to output an optimal result,
higher values will make the scheduling run faster with worse results.

**Running the Dijkstra Scheduler**

The `--dijkstra` flag will use the Dijstkra scheduler. Performing Dijkstra's algorithm it is
guaranteed to output the optimal scheduling given the constraints, however this mode can run very
slowly up to not completing at all on larger examples (such as
[`examples/permit_ma.balls`](./examples/permit_ma.balls)). Non-termination is especially likely when
using `--dijkstra` when the search is otherwise unconstrained.

**Constraining the search**

To speed up any of the above searches you may constrain the max stack depth that the program is
allowed to have at any point. The default value is the EVM's max stack depth of 1024. Too low of
a value may result in a stack-too-deep error. Constraining can allow Dijkstra to terminate in
reasonable times for larger examples such as `permit_ma.balls`.

Note that if the value is too low the scheduler may output a scheduling but it may not be the most
optimal possible schedule.

## Dependencies

BALLS is able to search for and create optimal stack schedules by going through and reordering
operations. To ensure that the code remains correct the system tracks "read" and "write"
dependencies. Some dependencies are quite straight forward to understand like `MEMORY` and
`STORAGE`. Having a "read" dependency means that you depend on it but that it does not matter in
what order it gets accessed so long as it does not get changed, "write" means that it affects the
dependency and that it's order has to remain fixed relative to other writes and to its preceding
e.g.

Original definition in code:
```
1. read A
2. write A
3. read A
4. read A
5. read A
6. write A
7. write A
```

Valid reordering:

```diff
1. read A
2. write A
+ 5. read A
+ 4. read A
+ 3. read A
6. write A
7. write A
```

**Invalid** reordering:

```diff
1. read A
- 3. read A
- 2. write A
4. read A
5. read A
- 7. write A
- 6. write A
```

The list of default dependencies, opcodes and their read/writes can be found under
[`src/transformer/std_evm.rs`](./src/transformer/std_evm.rs).


