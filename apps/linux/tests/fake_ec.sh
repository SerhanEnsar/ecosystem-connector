#!/bin/sh
# Stands in for `ec` in tests: prints what the real tool prints for a few commands.
case "$1" in
  pair)
    printf '    \342\226\210\342\226\200\342\226\204 \n'
    printf "Scan the code, or run this on the other device:\n\n  ec join 'ecpair:1?id=ab&s=cd&a=1.2.3.4:5'\n\n"
    printf 'Test (1234-ABCD) wants to pair.\nDoes the other device show the code 123 456? [y/N] '
    read -r answer
    if [ "$answer" = y ]; then echo "Paired with Test (1234-ABCD)."; else echo "Error: pairing failed: declined on this device"; exit 1; fi ;;
  send)
    printf 'Looking for Test on the network...\nHashing...\n\r   50.0%%  1 MB / 2 MB  1 MB/s\r  100.0%%  2 MB / 2 MB  2 MB/s    \n'
    echo "Sent 1 file(s), 2 MB in 1.0 s (2 MB/s over 1 path(s))." ;;
  *) echo "Error: unknown"; exit 2 ;;
esac
