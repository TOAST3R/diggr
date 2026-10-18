Store basket answers recorded on 2026-10-18 (store-carts task 1.1), trimmed to the fields the
extension reads; personal and session fields removed. The message in `bleep_already_in.json` is
reconstructed from its dotted key (the recorder showed `cart_item_already_added`). Juno's basket
page row, used for reads, is described in the change's design (D1) rather than stored: it is the
checkout form's `item[‹title›-‹product›]` field.
