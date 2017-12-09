# Design considerations

This is mostly lessons learned from developing [tagger](https://github.com/crumblingstatue/tagger).

## Design the application before you split it into libraries

`tagger` used a library called [tagmap](https://github.com/crumblingstatue/rust-tagmap),
which was basically developed before I decided how `tagger` will work.

In the end, `tagmap` didn't really work out.
Its architecture ended up limiting what `tagger` can do.

## The main screen is a thin view into the tag database.

`tagger` ended up cloning a lot of data (like the filename, tags., etc.)
for each image view on the main screen.
This didn't work out very well, especially for things like renaming, changing tags, etc.

Instead, each image in the databse should have its own unique ID,
by which it can always be uniquely identified.
The main screen's image elements would simply hold this ID, and not duplicate data.

This also holds true for runtime data, like whether the image loaded successfully,
texture location, etc.
This should also be held in a single database, a single entry for each unique ID.
Again, the main screen's image elements just hold the id that can be used to refer to this data.

Note that this ID should __always__ point to the same image, even between runs.
This is because we don't want to invaildate the on-disk thumbnail cache between runs.

This persitence, however, introduces a problem:
If we delete an existing image, it will introduce a "hole" in the id list, an unoccupied id.
If we keep adding and removing images, eventually the list will be full of unoccupied ids
that can't be reused.
This might not be a problem in practice though if we make the id datatype large enough.
If we make it 32 bit, it's unlikely that the user will ever add 4 billion images in total.
If the user added 1 image every second, it would take 136 years to occupy all ids.
There could potentially be a "compaction" feature, which would reshuffle the images to occupy
the unused ids, and forcibly regenerate the thumbnail cache.
Would this feature be actually needed in practice? Probably not.
