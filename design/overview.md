# Overview

Ever had a huge heap of images and wanted to quickly access the ones that interest you based on
certain properties? No? Ok.

Anyway, this is an application with the following features:

## Easily tag lots of images fast.

You just downloaded a bunch of images. You don't want to go through them one by one, and painstakingly tag each individual item by hand. What you want is the ability to select multiple images, and add the same tags to each.

## Fast filtering both by tags and name

Obviously, the main point of tagging is to allow to filter images based on certain properties defined by the user. This should also be as fast as possible. No stupid shit like reloading the same image twice just because the filtering requirements changed.

Oh, and image selection should persist between filterings. Meaning, if you have selected an image, and do a filter for other images, the selection should remain intact.

## Fast loading

If you have over 2000 images, you don't want to wait excruciating amounts of time waiting for them to load. This means a small thumbnail should be generated for each image, which can be quickly loaded compared to the large images. Obviously, this thumbnail cache should be saved to disk to persist between runs.

Loading should also be multithreaded, so loading an image doesn't block the UI.

## Advanced tagging system with the following features:

- aliases (e.g. `happy` = `cheerful` = `merry`)
- implies-relationship (e.g. `elephant` implies `pachyderm` implies `animal`)
- excluding tags (e.g. `elephant not:happy` or `animal not:elephant`)
- renaming tags. You might want to call it something different, or you made a spelling mistake, etc.

## Reordering images without renaming them.
The user might want to reorder images, for example comic pages are in the wrong order, and the user wants to specify the correct order without renaming the entries.

This could be accomplished by ordering images by id, and reordering would be done by swapping the ids of two entries.
