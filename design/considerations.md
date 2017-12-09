# Design considerations

This is mostly lessons learned from developing [tagger](https://github.com/crumblingstatue/tagger).

## Design the application before you split it into libraries

`tagger` used a library called [tagmap](https://github.com/crumblingstatue/rust-tagmap), which was basically developed before I decided how `tagger` will work.

In the end, `tagmap` didn't really work out. Its architecture ended up limiting what `tagger` can do.

## The main screen is a thin view into the tag database.

`tagger` ended up cloning a lot of data (like the filename, tags., etc.) for each image view on the
main screen. This didn't work out very well, especially for things like renaming, changing tags, etc.

Instead, each image in the databse should have its own unique ID, by which it can always be uniquely identified. The main screen's image elements would simply hold this ID, and not duplicate data.

This also holds true for runtime data, like whether the image loaded successfully, texture location, etc. This should also be held in a single database, a single entry for each unique ID. Again, the main screen's image elements just hold the id that can be used to refer to this data.