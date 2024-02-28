import re

STD_OPS = '''
dependency STORAGE
dependency TRANSIENT
dependency MEMORY
dependency MEMSIZE
dependency RECEIPT_LOGS
dependency BALANCES
dependency CODE
dependency RETURNDATA
dependency CONTROL_FLOW

////////////////////////////////////////////////////////////////
//                          PURE OPS                          //
////////////////////////////////////////////////////////////////
op add = stack(2, 1)
op mul = stack(2, 1)
op sub = stack(2, 1)
op div = stack(2, 1)
op sdiv = stack(2, 1)
op mod = stack(2, 1)
op smod = stack(2, 1)
op addmod = stack(3, 1)
op mulmod = stack(3, 1)
op exp = stack(2, 1)
op signextend = stack(2, 1)
op lt = stack(2, 1)
op gt = stack(2, 1)
op slt = stack(2, 1)
op sgt = stack(2, 1)
op eq = stack(2, 1)
op iszero = stack(1, 1)
op and = stack(2, 1)
op or = stack(2, 1)
op xor = stack(2, 1)
op not = stack(1, 1)
op byte = stack(2, 1)
op shl = stack(2, 1)
op shr = stack(2, 1)
op sar = stack(2, 1)

////////////////////////////////////////////////////////////////
//                         KECCAK-256                         //
////////////////////////////////////////////////////////////////
op sha3 = stack(2, 1) reads(MEMORY) writes(MEMSIZE)

////////////////////////////////////////////////////////////////
//                  CALL CONTEXT INSPECTORS                   //
////////////////////////////////////////////////////////////////
op address = stack(0, 1)
op balance = stack(1, 1) reads(BALANCES)
op origin = stack(0, 1)
op caller = stack(0, 1)
op callvalue = stack(0, 1)
op calldataload = stack(1, 1)
op calldatasize = stack(0, 1)
op calldatacopy = stack(3, 0) writes(MEMORY, MEMSIZE)
op gasprice = stack(0, 1)
op returndatasize = stack(0, 1) reads(RETURNDATA)
op returndatacopy = stack(3, 0) reads(RETURNDATA) writes(MEMORY, MEMSIZE)
op blockhash = stack(1, 1)
op coinbase = stack(0, 1)
op timestamp = stack(0, 1)
op number = stack(0, 1)
op prevrandao = stack(0, 1)
op gaslimit = stack(0, 1)
op chainid = stack(0, 1)
op selfbalance = stack(0, 1) reads(BALANCES)
op basefee = stack(0, 1)
op gas = stack(0, 1)

////////////////////////////////////////////////////////////////
//                        CODE GETTERS                        //
////////////////////////////////////////////////////////////////
op codesize = stack(0, 1)
op codecopy = stack(3, 0) writes(MEMORY, MEMSIZE)
op extcodesize = stack(1, 1) reads(CODE)
op extcodecopy = stack(4, 0) reads(CODE) writes(MEMORY, MEMSIZE)
op extcodehash = stack(1, 1) reads(CODE)

////////////////////////////////////////////////////////////////
//                           MEMORY                           //
////////////////////////////////////////////////////////////////
op msize = stack(0, 1) reads(MEMSIZE)
op mload = stack(1, 1) reads(MEMORY) writes(MEMSIZE)
op mstore = stack(2, 0) writes(MEMORY, MEMSIZE)
op mstore8 = stack(2, 0) writes(MEMORY, MEMSIZE)

////////////////////////////////////////////////////////////////
//                     PERSISTENT STORAGE                     //
////////////////////////////////////////////////////////////////
op sload = stack(1, 1) reads(STORAGE)
op sstore = stack(2, 0) reads(CONTROL_FLOW) writes(STORAGE)

////////////////////////////////////////////////////////////////
//                     TRANSIENT STORAGE                      //
////////////////////////////////////////////////////////////////
op tload = stack(1, 1) reads(TRANSIENT)
op tstore = stack(2, 0) reads(CONTROL_FLOW) writes(TRANSIENT)


////////////////////////////////////////////////////////////////
//                           JUMPS                            //
////////////////////////////////////////////////////////////////
op jump = stack(1, 0) writes(CONTROL_FLOW)
op jumpi = stack(2, 0) writes(CONTROL_FLOW)


    ////////////////////////////////////////////////////////////////
    //                           EVENTS                           //
    ////////////////////////////////////////////////////////////////
op log0 = stack(2, 0) reads(CONTROL_FLOW, MEMORY) writes(RECEIPT_LOGS, MEMSIZE)
op log1 = stack(3, 0) reads(CONTROL_FLOW, MEMORY) writes(RECEIPT_LOGS, MEMSIZE)
op log2 = stack(4, 0) reads(CONTROL_FLOW, MEMORY) writes(RECEIPT_LOGS, MEMSIZE)
op log3 = stack(5, 0) reads(CONTROL_FLOW, MEMORY) writes(RECEIPT_LOGS, MEMSIZE)
op log4 = stack(6, 0) reads(CONTROL_FLOW, MEMORY) writes(RECEIPT_LOGS, MEMSIZE)


////////////////////////////////////////////////////////////////
//                      CALLS & CREATION                      //
////////////////////////////////////////////////////////////////
op call         = stack(7, 1) reads(CONTROL_FLOW)       writes(CODE, BALANCES, STORAGE, TRANSIENT, MEMORY, MEMSIZE, RETURNDATA)
op callcode     = stack(7, 1) reads(CONTROL_FLOW)       writes(CODE, BALANCES, STORAGE, TRANSIENT, MEMORY, MEMSIZE, RETURNDATA)
op delegatecall = stack(6, 1) reads(CONTROL_FLOW)       writes(CODE, BALANCES, STORAGE, TRANSIENT, MEMORY, MEMSIZE, RETURNDATA)
op staticcall   = stack(6, 1) reads(CODE, BALANCES, STORAGE, TRANSIENT) writes(MEMORY, MEMSIZE, RETURNDATA)
op create  = stack(3, 1) reads(MEMORY, CONTROL_FLOW) writes(CODE, BALANCES, STORAGE, TRANSIENT, MEMSIZE, RETURNDATA)
op create2 = stack(4, 1) reads(MEMORY, CONTROL_FLOW) writes(CODE, BALANCES, STORAGE, TRANSIENT, MEMSIZE, RETURNDATA)


////////////////////////////////////////////////////////////////
//                        TERMINATION                         //
////////////////////////////////////////////////////////////////
op stop = stack(0, 0) writes(CONTROL_FLOW)
op return = stack(2, 0) writes(CONTROL_FLOW)
op revert = stack(2, 0) reads(CONTROL_FLOW)
op invalid = stack(0, 0) reads(CONTROL_FLOW)
op selfdestruct = stack(1, 0) writes(CONTROL_FLOW)

'''


def read_dep_list(raw_list: str | None) -> list[str]:
    if raw_list is None:
        return []
    head, tail = raw_list.split('(', 1)
    mid, tail = tail.split(')', 1)
    return [
        item.strip()
        for item in mid.split(',')
    ]


def parse_op(rest: str) -> tuple[str, int, str, list[str], list[str]]:
    reads = []
    writes = []

    match = re.match(
        r'(\w+)\s+=\s+stack\((\d+),\s+(\d+)\)(\s+reads\((?:\w+,\s+)*\w+\))?(\s+writes\((?:\w+, )*\w+\))?',
        rest
    )
    assert match is not None
    name, stack_in, stack_out, reads, writes = match.groups()
    stack_out = int(stack_out)
    assert stack_out in range(0, 2)
    return name, int(stack_in), str(bool(stack_out)).lower(), read_dep_list(reads), read_dep_list(writes)


def main():
    lines = [
        line.strip()
        for line in STD_OPS.strip().splitlines()
        if line.strip()
    ]
    deps = []
    ops = []
    for line in lines:
        if line.startswith('//'):
            ops.append(line)
            continue
        t, rest = line.split(' ', 1)
        if t == 'dependency':
            deps.append(rest)
        else:
            assert t == 'op'
            name, stack_in, stack_out, reads, writes = parse_op(rest)
            if not reads and not writes:
                op = f'Op::pure("{name}", {stack_in}, {stack_out})'
            else:
                reads_vec = ', '.join(f'"{read}"' for read in reads)
                writes_vec = ', '.join(f'"{write}"' for write in writes)
                op = f'Op::new("{name}", {stack_in}, {stack_out}, vec![{reads_vec}], vec![{writes_vec}])'
            ops.append(op)
    dep_list = ','.join([f'"{dep}"' for dep in deps])
    print(f'let dependencies = vec![{dep_list}];')
    ops_list = ',\n'.join(ops)
    print(f'let ops = vec![\n{ops_list}\n];')


if __name__ == '__main__':
    main()
