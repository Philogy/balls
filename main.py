from balls.parser import parse


program = '''
// Basic comment

// Assign variable selector to result of some operations
// Semi-colon ends line.
word = calldataload(0);
// Use hex literal
selector = shr(0xe0, word);

selector_is_transfer_from = eq(selector, 0x23b872dd);

// Conditional goto
goto transfer_from if selector_is_transfer_from;

// Unconditional goto
goto error;

// Label definition line doesn't end in semi-colon
transfer_from:

from = calldataload(0x04);
to = calldataload(0x24);
amount = calldataload(0x44);

mstore(0x00, from);
mstore(0x20, caller());

allowance_slot = keccak256(0x00, 0x40);

allowance = sload(allowance_slot);

// goto condition dictated by 
goto sufficient_allowance if byte(0, allowance);

	goto error if gt(amount, allowance);
	sstore(allowance_slot, sub(allowance, amount));

sufficient_allowance:

from_balance = sload(from);

goto error if gt(amount, from_balance);

sstore(from, sub(from_balance, amount));
sstore(to, add(sload(to), amount));

mstore(0x00, 1);
return(0x00, 0x20);

// Label definition can be on same line as operations
error: revert(0, 0);

'''


def main():
    out = parse(program)
    print(out.pretty())

    return out


if __name__ == '__main__':
    out = main()
