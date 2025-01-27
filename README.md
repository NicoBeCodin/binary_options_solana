# Binary options

The goal of this project is to implement an onchain program to offer binary options on different assets, with everything happening on the solana chain.

## Work in progress

This is project is only the start so don't expect safe code, I'm doing this for fun.

### What's being traded ?

Think of the binary options as polymarket-style shares, where to a question such as "Will SOL be over $250 on XX/XX 12:00 EST ? ", Yes and no shares are being traded. In theory, binary options price reflect reflect the delta of traditional european option.

### How are markets settled ?

Markets are settled using the pyth on chain price feed, for the moment only three types of assets are offered, but this will change in the future.

### TO-DO

 - Make a funcitonning resolve_market and place_bet
 - Create SPL tokens
 - Implement a way for supply to meet demand on chain (Dutch Auction or peer to peer but requires more users)
 - Add more assets
 - Review code to make it safer

# binary_options_solana
