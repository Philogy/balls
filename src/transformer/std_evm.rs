#[derive(Clone, Debug)]
pub struct Op {
    pub ident: String,
    pub stack_in: u16,
    pub stack_out: bool,
    pub reads: Vec<String>,
    pub writes: Vec<String>,
    pub other: Option<(String, Vec<usize>)>,
}

impl Op {
    fn new(
        ident: &str,
        stack_in: u16,
        stack_out: bool,
        reads: Vec<&str>,
        writes: Vec<&str>,
    ) -> Self {
        Self {
            ident: ident.to_string(),
            stack_in,
            stack_out,
            reads: reads.into_iter().map(|s| s.to_string()).collect(),
            writes: writes.into_iter().map(|s| s.to_string()).collect(),
            other: None,
        }
    }

    fn pure(ident: &str, stack_in: u16, stack_out: bool) -> Self {
        Self::new(ident, stack_in, stack_out, vec![], vec![])
    }

    /// Commutative function with two arguments, implicitly pure (no reads/writes)
    fn two_comm(ident: &str) -> Self {
        Self {
            ident: ident.to_string(),
            stack_in: 2,
            stack_out: true,
            reads: vec![],
            writes: vec![],
            other: Some((ident.to_string(), vec![1, 0])),
        }
    }

    fn chiral(ident: &str, other: &str, stack_in: u16, indices: Vec<usize>) -> Self {
        assert!(
            indices.len() == stack_in.into(),
            "Indices {:?} stack_in {} mismatch",
            indices,
            stack_in
        );
        let mut was_found = vec![false; stack_in.into()];
        for i in &indices {
            *was_found
                .get_mut(*i)
                .unwrap_or_else(|| panic!("Invalid operand index {}", i)) = true;
        }
        assert!(
            was_found.iter().all(|b| *b),
            "Duplicate indicies in {:?}",
            indices
        );
        Self {
            ident: ident.to_string(),
            stack_in,
            stack_out: true,
            reads: vec![],
            writes: vec![],
            other: Some((other.to_string(), indices)),
        }
    }
}

pub fn get_standard_opcodes_and_deps() -> (Vec<&'static str>, Vec<Op>) {
    let dependencies = vec![
        "STORAGE",
        "TRANSIENT",
        "MEMORY",
        "MEMSIZE",
        "RECEIPT_LOGS",
        "BALANCES",
        "CODE",
        "RETURNDATA",
        "CONTROL_FLOW",
    ];
    let ops = vec![
        ////////////////////////////////////////////////////////////////,
        //                          PURE OPS                          //,
        ////////////////////////////////////////////////////////////////,
        Op::two_comm("add"),
        Op::two_comm("mul"),
        Op::pure("sub", 2, true),
        Op::pure("div", 2, true),
        Op::pure("sdiv", 2, true),
        Op::pure("mod", 2, true),
        Op::pure("smod", 2, true),
        Op::chiral("addmod", "addmod", 3, vec![1, 0, 2]),
        Op::chiral("mulmod", "mulmod", 3, vec![1, 0, 2]),
        Op::pure("exp", 2, true),
        Op::pure("signextend", 2, true),
        Op::chiral("lt", "gt", 2, vec![1, 0]),
        Op::chiral("gt", "lt", 2, vec![1, 0]),
        Op::chiral("slt", "sgt", 2, vec![1, 0]),
        Op::chiral("sgt", "slt", 2, vec![1, 0]),
        Op::two_comm("eq"),
        Op::pure("iszero", 1, true),
        Op::two_comm("and"),
        Op::two_comm("or"),
        Op::two_comm("xor"),
        // Aliased into sub in the huff formatter,
        // TODO: Generalize
        Op::two_comm("diff"),
        Op::pure("not", 1, true),
        Op::pure("byte", 2, true),
        Op::pure("shl", 2, true),
        Op::pure("shr", 2, true),
        Op::pure("sar", 2, true),
        ////////////////////////////////////////////////////////////////,
        //                         KECCAK-256                         //,
        ////////////////////////////////////////////////////////////////,
        Op::new("sha3", 2, true, vec!["MEMORY"], vec!["MEMSIZE"]),
        ////////////////////////////////////////////////////////////////,
        //                  CALL CONTEXT INSPECTORS                   //,
        ////////////////////////////////////////////////////////////////,
        Op::pure("address", 0, true),
        Op::new("balance", 1, true, vec!["BALANCES"], vec![]),
        Op::pure("origin", 0, true),
        Op::pure("caller", 0, true),
        Op::pure("callvalue", 0, true),
        Op::pure("calldataload", 1, true),
        Op::pure("calldatasize", 0, true),
        Op::new("calldatacopy", 3, false, vec![], vec!["MEMORY", "MEMSIZE"]),
        Op::pure("gasprice", 0, true),
        Op::new("returndatasize", 0, true, vec!["RETURNDATA"], vec![]),
        Op::new(
            "returndatacopy",
            3,
            false,
            vec!["RETURNDATA"],
            vec!["MEMORY", "MEMSIZE"],
        ),
        Op::pure("blockhash", 1, true),
        Op::pure("coinbase", 0, true),
        Op::pure("timestamp", 0, true),
        Op::pure("number", 0, true),
        Op::pure("prevrandao", 0, true),
        Op::pure("gaslimit", 0, true),
        Op::pure("chainid", 0, true),
        Op::new("selfbalance", 0, true, vec!["BALANCES"], vec![]),
        Op::pure("basefee", 0, true),
        Op::pure("gas", 0, true),
        ////////////////////////////////////////////////////////////////,
        //                        CODE GETTERS                        //,
        ////////////////////////////////////////////////////////////////,
        Op::pure("codesize", 0, true),
        Op::new("codecopy", 3, false, vec![], vec!["MEMORY", "MEMSIZE"]),
        Op::new("extcodesize", 1, true, vec!["CODE"], vec![]),
        Op::new(
            "extcodecopy",
            4,
            false,
            vec!["CODE"],
            vec!["MEMORY", "MEMSIZE"],
        ),
        Op::new("extcodehash", 1, true, vec!["CODE"], vec![]),
        ////////////////////////////////////////////////////////////////,
        //                           MEMORY                           //,
        ////////////////////////////////////////////////////////////////,
        Op::new("msize", 0, true, vec!["MEMSIZE"], vec![]),
        Op::new("mload", 1, true, vec!["MEMORY"], vec!["MEMSIZE"]),
        Op::new("mstore", 2, false, vec![], vec!["MEMORY", "MEMSIZE"]),
        Op::new("mstore8", 2, false, vec![], vec!["MEMORY", "MEMSIZE"]),
        ////////////////////////////////////////////////////////////////,
        //                     PERSISTENT STORAGE                     //,
        ////////////////////////////////////////////////////////////////,
        Op::new("sload", 1, true, vec!["STORAGE"], vec![]),
        Op::new("sstore", 2, false, vec!["CONTROL_FLOW"], vec!["STORAGE"]),
        ////////////////////////////////////////////////////////////////,
        //                     TRANSIENT STORAGE                      //,
        ////////////////////////////////////////////////////////////////,
        Op::new("tload", 1, true, vec!["TRANSIENT"], vec![]),
        Op::new("tstore", 2, false, vec!["CONTROL_FLOW"], vec!["TRANSIENT"]),
        ////////////////////////////////////////////////////////////////,
        //                           JUMPS                            //,
        ////////////////////////////////////////////////////////////////,
        Op::new("jump", 1, false, vec![], vec!["CONTROL_FLOW"]),
        Op::new("jumpi", 2, false, vec![], vec!["CONTROL_FLOW"]),
        ////////////////////////////////////////////////////////////////,
        //                           EVENTS                           //,
        ////////////////////////////////////////////////////////////////,
        Op::new(
            "log0",
            2,
            false,
            vec!["CONTROL_FLOW", "MEMORY"],
            vec!["RECEIPT_LOGS", "MEMSIZE"],
        ),
        Op::new(
            "log1",
            3,
            false,
            vec!["CONTROL_FLOW", "MEMORY"],
            vec!["RECEIPT_LOGS", "MEMSIZE"],
        ),
        Op::new(
            "log2",
            4,
            false,
            vec!["CONTROL_FLOW", "MEMORY"],
            vec!["RECEIPT_LOGS", "MEMSIZE"],
        ),
        Op::new(
            "log3",
            5,
            false,
            vec!["CONTROL_FLOW", "MEMORY"],
            vec!["RECEIPT_LOGS", "MEMSIZE"],
        ),
        Op::new(
            "log4",
            6,
            false,
            vec!["CONTROL_FLOW", "MEMORY"],
            vec!["RECEIPT_LOGS", "MEMSIZE"],
        ),
        ////////////////////////////////////////////////////////////////,
        //                      CALLS & CREATION                      //,
        ////////////////////////////////////////////////////////////////,
        Op::new(
            "call",
            7,
            true,
            vec!["CONTROL_FLOW"],
            vec![
                "CODE",
                "BALANCES",
                "STORAGE",
                "TRANSIENT",
                "MEMORY",
                "MEMSIZE",
                "RETURNDATA",
            ],
        ),
        Op::new(
            "callcode",
            7,
            true,
            vec!["CONTROL_FLOW"],
            vec![
                "CODE",
                "BALANCES",
                "STORAGE",
                "TRANSIENT",
                "MEMORY",
                "MEMSIZE",
                "RETURNDATA",
            ],
        ),
        Op::new(
            "delegatecall",
            6,
            true,
            vec!["CONTROL_FLOW"],
            vec![
                "CODE",
                "BALANCES",
                "STORAGE",
                "TRANSIENT",
                "MEMORY",
                "MEMSIZE",
                "RETURNDATA",
            ],
        ),
        Op::new(
            "staticcall",
            6,
            true,
            vec!["CODE", "BALANCES", "STORAGE", "TRANSIENT"],
            vec!["MEMORY", "MEMSIZE", "RETURNDATA"],
        ),
        Op::new(
            "create",
            3,
            true,
            vec!["MEMORY", "CONTROL_FLOW"],
            vec!["CODE", "BALANCES", "STORAGE", "TRANSIENT", "MEMSIZE"],
        ),
        Op::new(
            "create2",
            4,
            true,
            vec!["MEMORY", "CONTROL_FLOW"],
            vec!["CODE", "BALANCES", "STORAGE", "TRANSIENT", "MEMSIZE"],
        ),
        ////////////////////////////////////////////////////////////////,
        //                        TERMINATION                         //,
        ////////////////////////////////////////////////////////////////,
        Op::new("stop", 0, false, vec![], vec!["CONTROL_FLOW"]),
        Op::new("return", 2, false, vec![], vec!["CONTROL_FLOW"]),
        Op::new("revert", 2, false, vec!["CONTROL_FLOW"], vec![]),
        Op::new("invalid", 0, false, vec!["CONTROL_FLOW"], vec![]),
        Op::new(
            "selfdestruct",
            1,
            false,
            vec![],
            vec!["BALANCES", "CONTROL_FLOW"],
        ),
    ];

    for op in &ops {
        for read in &op.reads {
            assert!(
                dependencies.iter().any(|dep| *dep == read),
                "{} has invalid read dependency {}",
                op.ident,
                read
            );
        }
        for w in &op.reads {
            assert!(
                dependencies.iter().any(|dep| *dep == w),
                "{} has invalid write dependency {}",
                op.ident,
                w
            );
        }
    }

    (dependencies, ops)
}
