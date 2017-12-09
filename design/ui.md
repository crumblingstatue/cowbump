# Main screen

The main screen is simply a grid of images.

Left clicking an image opens it with the currently set external image viewer.
Right clicking an image brings up the `meta overlay`.

Shift left clicking an image adds/removes it from the selection.
Ctrl + A selects all filtered images.
Ctrl + shift + A clears selection.
Shift right clicking opens the `multi meta overlay`

Pressing `/` brings up the `filter editor overlay`.

## meta overlay
View and edit the image's filename and tags.

## multi meta overlay

Shows how many images are currently selected, and shows the tags that are common to all selected images. You can then add/remove common tags.

## filter editor overlay

Shows the current filter string. The user can type to edit it.
Finishing a word (adding whitespace or deleting a word completely) updates the filter immediately. Pressing `return` also updates the filter, and closes the filter editor. Pressing escape at any time resets the filtering state to before the filter editor was opened, and closes the filter editor.