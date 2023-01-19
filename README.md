# Wordgames

A Rust implementation of [PixelSam123's Java wordgames](https://github.com/PixelSam123/wordgames4j).

Word bank retrieved from [Datamuse](https://www.datamuse.com/api/).

I worked on this as a learning project, so things may be subpar.

---

## Spinning it up

Simply run the binary, or `cargo run`

## Demo

This also works with [PixelSam123's client](https://pixelsam123.github.io/minigames). Put `wss://play.norin.me/{ROUTE}` in the server URL box.

## Available routes

1. `ws/anagram` Normal anagrams game. Plans:
   - [ ] Multiple language support
   - [ ] Time configuration
   - [ ] Word length configuration
   - [ ] Timer configuration
