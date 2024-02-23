use crate::parser::ast::Ast;
use crate::parser::Spanned;

use crate::parser::{error_printing::print_errors, lexer, parser, types::resolve_span_span};

pub fn get_std() -> Vec<Spanned<Ast>> {
    let src = "
////////////////////////////////////////////////////////////////
//                        DEPENDENCIES                        //
////////////////////////////////////////////////////////////////
#define dependency STORAGE
#define dependency TRANSIENT
#define dependency MEMORY
#define dependency MEMSIZE
#define dependency RECEIPT_LOGS
#define dependency BALANCES
#define dependency CODE
#define dependency RETURNDATA
#define dependency CONTROL_FLOW

////////////////////////////////////////////////////////////////
//                     ARITHMETIC & LOGIC                     //
////////////////////////////////////////////////////////////////
#define op add = stack(2, 1)
#define op mul = stack(2, 1)
#define op sub = stack(2, 1)
#define op div = stack(2, 1)
#define op sdiv = stack(2, 1)
#define op mod = stack(2, 1)
#define op smod = stack(2, 1)
#define op addmod = stack(3, 1)
#define op mulmod = stack(3, 1)
#define op exp = stack(2, 1)
#define op signextend = stack(2, 1)
#define op lt = stack(2, 1)
#define op gt = stack(2, 1)
#define op slt = stack(2, 1)
#define op sgt = stack(2, 1)
#define op eq = stack(2, 1)
#define op iszero = stack(1, 1)
#define op and = stack(2, 1)
#define op or = stack(2, 1)
#define op xor = stack(2, 1)
#define op not = stack(1, 1)
#define op byte = stack(2, 1)
#define op shl = stack(2, 1)
#define op shr = stack(2, 1)
#define op sar = stack(2, 1)

////////////////////////////////////////////////////////////////
//                           CRYPTO                           //
////////////////////////////////////////////////////////////////
#define op sha3 = stack(2, 1) reads(MEMORY) writes(MEMSIZE)

#define op address = stack(0, 1)
#define op balance = stack(1, 1) reads(BALANCES)
#define op origin = stack(0, 1)
#define op caller = stack(0, 1)
#define op callvalue = stack(0, 1)
#define op calldataload = stack(1, 1)
#define op calldatasize = stack(0, 1)
#define op calldatacopy = stack(3, 0) writes(MEMORY, MEMSIZE)
#define op codesize = stack(0, 1)
#define op codecopy = stack(3, 0) writes(MEMORY, MEMSIZE)
#define op gasprice = stack(0, 1)
#define op extcodesize = stack(1, 1) reads(CODE)
#define op extcodecopy = stack(4, 0) reads(CODE) writes(MEMORY, MEMSIZE)
#define op returndatasize = stack(0, 1) reads(RETURNDATA)
#define op returndatacopy = stack(3, 0) reads(RETURNDATA) writes(MEMORY, MEMSIZE)
#define op extcodehash = stack(1, 1) reads(CODE)
#define op blockhash = stack(1, 1)
#define op coinbase = stack(0, 1)
#define op timestamp = stack(0, 1)
#define op number = stack(0, 1)
#define op prevrandao = stack(0, 1)
#define op gaslimit = stack(0, 1)
#define op chainid = stack(0, 1)
#define op selfbalance = stack(0, 1) reads(BALANCES)
#define op basefee = stack(0, 1)
#define op gas = stack(0, 1)
#define op pop = stack(1, 0)

#define op msize = stack(0, 1) reads(MEMSIZE)
#define op mload = stack(1, 1) reads(MEMORY) writes(MEMSIZE)
#define op mstore = stack(2, 0) writes(MEMORY, MEMSIZE)
#define op mstore8 = stack(2, 0) writes(MEMORY, MEMSIZE)

#define op sload = stack(1, 1) reads(STORAGE)
#define op sstore = stack(2, 0) reads(CONTROL_FLOW) writes(STORAGE)

#define op tload = stack(1, 1) reads(TRANSIENT)
#define op tstore = stack(2, 0) reads(CONTROL_FLOW) writes(TRANSIENT)

#define op jump = stack(1, 0) writes(CONTROL_FLOW)
#define op jumpi = stack(2, 0) writes(CONTROL_FLOW)

#define op log0 = stack(2, 0) reads(CONTROL_FLOW, MEMORY) writes(RECEIPT_LOGS, MEMSIZE)
#define op log1 = stack(3, 0) reads(CONTROL_FLOW, MEMORY) writes(RECEIPT_LOGS, MEMSIZE)
#define op log2 = stack(4, 0) reads(CONTROL_FLOW, MEMORY) writes(RECEIPT_LOGS, MEMSIZE)
#define op log3 = stack(5, 0) reads(CONTROL_FLOW, MEMORY) writes(RECEIPT_LOGS, MEMSIZE)
#define op log4 = stack(6, 0) reads(CONTROL_FLOW, MEMORY) writes(RECEIPT_LOGS, MEMSIZE)

#define op call         = stack(7, 1) reads(MEMORY, CONTROL_FLOW)       writes(CODE, BALANCES, STORAGE, TRANSIENT, MEMSIZE, RETURNDATA)
#define op callcode     = stack(7, 1) reads(MEMORY, CONTROL_FLOW)       writes(CODE, BALANCES, STORAGE, TRANSIENT, MEMSIZE, RETURNDATA)
#define op delegatecall = stack(6, 1) reads(MEMORY, CONTROL_FLOW)       writes(CODE, BALANCES, STORAGE, TRANSIENT, MEMSIZE, RETURNDATA)
#define op staticcall   = stack(6, 1) reads(MEMORY, CODE, BALANCES, STORAGE, TRANSIENT) writes(MEMSIZE, RETURNDATA)

#define op create  = stack(3, 1) reads(MEMORY, CONTROL_FLOW) writes(CODE, BALANCES, STORAGE, TRANSIENT, MEMSIZE, RETURNDATA)
#define op create2 = stack(4, 1) reads(MEMORY, CONTROL_FLOW) writes(CODE, BALANCES, STORAGE, TRANSIENT, MEMSIZE, RETURNDATA)

////////////////////////////////////////////////////////////////
//                        TERMINATION                         //
////////////////////////////////////////////////////////////////
#define op stop = stack(0, 0) writes(CONTROL_FLOW)
#define op return = stack(2, 0) writes(CONTROL_FLOW)
#define op revert = stack(2, 0) reads(CONTROL_FLOW)
#define op invalid = stack(0, 0) reads(CONTROL_FLOW)
#define op selfdestruct = stack(1, 0) writes(CONTROL_FLOW)";

    let lex_out = lexer::lex(src);

    // TODO: Proper lexer error handling
    let spanned_tokens = lex_out.unwrap();

    let tokens: Vec<_> = spanned_tokens.iter().map(|t| t.inner.clone()).collect();

    let (maybe_ast_nodes, errs) = parser::parse_tokens(tokens.clone());

    let errored = print_errors(&src, "std_evm.balls", errs, |tok_span| {
        resolve_span_span(tok_span, &spanned_tokens)
    });
    assert!(!errored, "std_evm errored");

    maybe_ast_nodes.unwrap()
}
