# Bytecode Assembler \w Low-Level Scheduling (BALLS)

This repo is meant to be a proof of concept, investigating whether it's feasible to use exhaustive
searching for optimal stack scheduling for EVM programs.

Stack scheduling is the process of turning a sequence of variable assignments into stack
manipulating instructions. Unlike traditional, register-based ISAs, the EVMs stack based nature
doesn't map very well to assignments.

## Guide

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
a value may result in a stack-too-deep error.

Note that if the value is too low the scheduler may output a scheduling but it may not be the most
optimal possible schedule.
