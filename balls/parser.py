from lark import Lark

GRAMMAR = r'''
    %import common.WS
    %import common.CPP_COMMENT
    %import common.C_COMMENT
    %import common.CNAME
    %import common.INT
    %import common.HEXDIGIT

    start: statement*

    ?statement: assignment
              | direct_invocation
              | goto
              | label_definition

    assignment: identifier "=" expression ";"
    direct_invocation: operation ";"
    goto: "goto" identifier ("if" expression)? ";"
    label_definition: identifier ":"

    expression: identifier
              | literal
              | operation

    operation: opcode argument_list

    argument_list: "(" (argument ",")* argument? ")"
    argument: expression

    // Disallow stack manipulation opcodes (swaps, dups), literal pushes (1-32) and control flow
    // instructions (jump, jumpi)
    opcode: "stop"i | "add"i | "mul"i | "sub"i | "div"i | "sdiv"i | "mod"i | "smod"i | "addmod"i
          | "mulmod"i | "exp"i | "signextend"i | "lt"i | "gt"i | "slt"i | "sgt"i | "eq"i
          | "iszero"i | "and"i | "or"i | "xor"i | "not"i | "byte"i | "shl"i | "shr"i | "sar"i
          | "sha3"i | "keccak256"i | "address"i | "balance"i | "origin"i | "caller"i | "callvalue"i
          | "calldataload"i | "calldatasize"i | "calldatacopy"i | "codesize"i | "codecopy"i
          | "gasprice"i | "extcodesize"i | "extcodecopy"i | "returndatasize"i | "returndatacopy"i
          | "extcodehash"i | "blockhash"i | "coinbase"i | "timestamp"i | "number"i | "prevrandao"i
          | "gaslimit"i | "chainid"i | "selfbalance"i | "basefee"i | "pop"i | "mload"i | "mstore"i
          | "mstore8"i | "sload"i | "sstore"i | "msize"i | "gas"i | "push0"i | "log0"i | "log1"i
          | "log2"i | "log3"i | "log4"i | "create"i | "call"i | "callcode"i | "return"i
          | "delegatecall"i | "create2"i | "staticcall"i | "revert"i | "invalid"i | "selfdestruct"i

    identifier: CNAME

    literal: DIRECT_LITERAL
    DIRECT_LITERAL: INT | HEXADECIMAL | BINARY

    HEXADECIMAL: "0x" HEXDIGIT+
    BINARY: "0b" BINDIGIT+
    BINDIGIT: "0" | "1"

    %ignore WS
    %ignore CPP_COMMENT
    %ignore C_COMMENT
'''


def parse(s: str):
    return Lark(GRAMMAR).parse(s)
