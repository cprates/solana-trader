
## Notes
* Token account from where the offer accout is taken is has it's authority changed to the trader program while the trade is available, and returned when the trade is done. This wasn't a good idea because, for example, the user needs to be careful to avoid using an ATA that could be used somewhere else.
* The offer amount is defined by the tokan balance of the offer src account. This is possible because of the above ^
* Taker pays for creation of the ATA fee acount if it doesn't exists
* When the trade is done a fee is taken from the user how created the trade into an ATA owned by the creator of the trader program
* To make sure the trade fee goes to the correct account, the program has its authority account hardcoded in order to be able to compare with the one passed in the instruction
* The trade fee percentage is hardcoded in the program
* The trade fee is transfered to an ATA account owned by the program authority, and are created if doesn't exist when the trade is accepted.

## Steps to test

Create 3 wallets, wallet0 as the authority of the program, wallet1 as user A (offer), wallet2 as user B (trade)

```bash
solana-keygen new --outfile wallet0.json
solana config set --keypair $(pwd)/wallet0.json
solana airdrop 50000
solana-keygen new --outfile wallet1.json
solana config set --keypair $(pwd)/wallet1.json
solana airdrop 50000
solana-keygen new --outfile wallet2.json
solana config set --keypair $(pwd)/wallet2.json
solana airdrop 50000
```

Update the program code in `processor.rs/PROGRAM_AUTHORITY` with the address of wallet0.


Build and upload the program
```bash
solana config set --keypair $(pwd)/wallet0.json
cargo build-bpf --bpf-out-dir=./ && solana program deploy $(pwd)/trader.so
```


Configure program with its wallet address and its own program id
```bash
cargo run -- config -w <WALLET> -p <PROGRAM>
```


One can create all the Mint and Token accounts by hand or use the bootstrap option
```bash
cargo run -- bootstrap $(pwd)/../wallet1.json $(pwd)/../wallet2.json
```


Now switch to User A and create a trade. `OFFER_ACCOUNT` is the offer src account. Note that the amount passed is the trade amount and not the offer amount. The offer amout is the tokan balance of the `OFFER_ACCOUNT`.
```
solana config set --keypair $(pwd)/../wallet1.json
cargo run -- create <OFFER_ACCOUNT> <TRADE_TOKEN> <TRADE_AMOUNT>
```


Accept the trade with User B. `OFFER_OWNER` is the public address of the wallet1. The command below does not specify the destination accounts (offer dst and trade dst). In this case ATA accounts are created.
```
solana config set --keypair $(pwd)/../wallet2.json
cargo run -- trade <TRADE_ID> <OFFER_SRC> <OFFER_AMOUNT> <TRADE_SRC> <TRADE_AMOUNT> <OFFER_OWNER>
```

