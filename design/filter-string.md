# The filter string

The filter string specifies how the images are filtered.

## Requirements

- Easy and fast way to specify tags. This likely means tags are separated by whitespace.
- Ability to select images that have no tags. This likely means adding some kind of meta-tagging ability, where you can say for example `:no-tags`, which has a special meaning instead of being a tag itself.
- Ability to filter images by filename. This also likely means having a meta-tag system. Perhaps `:filename=foo`. This would probably also warrant an ability to have spaces in a tag. Perhaps by quoting, e.g. `:filename="foo bar"`.
- An empty string must mean "don't filter at all".
