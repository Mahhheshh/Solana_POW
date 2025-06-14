### Native implementation for escrow pinocchio

#### base requirements:
- user shall be allowed to send native sol in a pda.
- and assign the send authority to a pubkey
- the pubkey shall be allowed to send the native sol balance to the receiver pubkey
- the program shall close after the last transfer, sending sol to receiver and rent to the creator
