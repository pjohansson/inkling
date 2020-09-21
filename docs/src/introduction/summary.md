# Introduction

When writing a story for a game one has to consider how to get the script into it.
This is simple if the script is in plain text, but depending on our game we might
need to account for many possibilities, including but not limited to:

*   Using variables for names or items which are declared elsewhere 
*   Branching the story into different paths
*   Testing conditions for presenting certain parts
*   Marking up content for emphasis or effect

`Ink` is a scripting language which implements many of these features in its design.
`inkling` is a library which can read this language and present the final text 
to the reader. This chapter will introduce what the language is and how `inkling` 
works with it.