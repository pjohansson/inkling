# Missing features

This page lists notable features of `Ink` which are currently missing in `inkling`.
Some may be implemented, others will be more difficult. 

## Variable assignment

Assigning new values to [variables](variables.md) in the script.

```plain
~ rank = "Capitaine"
~ coins = coins + 4
```

## Including other files

Dividing the script into several files and including them in the preamble 
of the main script.

```plain
INCLUDE château.ink
INCLUDE gloomwood.ink
```

## Multiline comments

Using `/*` and `*/` markers to begin and end multiline comments.

```plain
Line one /* We can use multiline comments
            to split them over several lines, 
            which may aid readability. */
Line two
```

## Multiline conditionals

Using multiline blocks to create larger if-else or switch statements.

```plain
{condition:
    if condition content
- else:
    else this content
}
```

```plain
{
    - condition1:
        if condition 1 content
    - condition2:
        else if condition 2 content
    - else:
        else this content
}
```

## Labels

Add [labels](https://github.com/inkle/ink/blob/master/Documentation/WritingWithInk.md#gathers-and-options-can-be-labelled) 
to choices and gather points to refer and divert to them.

```plain
*   (one) Choice
*   (two) Choice
-   (gather) Gather 
```

## Functions

Calling various types of functions from the script.

### Built-in functions

Support for the pre-defined functions of `Ink`.

```plain
~ result = RANDOM(1, 6)
~ result = POW(3, 2)
```

### Definining functions

Defining functions in the script to print text, modify variables and return calculations.

```plain
// Modifying referenced values
=== function add(ref x, value) ===
~ x = x + value
```

```plain
// Modifying global variables
VAR coins = 0
=== function add_wealth(v) ===
~ coins = coins + v
```

```plain
// Writing text 
=== function greet(person) ===
Greetings, {person}!
```

### External functions

Begin able to call external Rust functions from the script.

## Threads

[More information.](https://github.com/inkle/ink/blob/master/Documentation/WritingWithInk.md#2-threads)

## Tunnels

[More information.](https://github.com/inkle/ink/blob/master/Documentation/WritingWithInk.md#1-tunnels)

## Advanced state tracking

[More information.](https://github.com/inkle/ink/blob/master/Documentation/WritingWithInk.md#part-5-advanced-state-tracking)