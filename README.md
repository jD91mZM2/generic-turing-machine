# Generic turing machine

A turing machine with generics, because why not?

## Problem

Turing machines have two kinds of states:
 - The state itself
 - The buffer

That means when you actually manipulate the buffer, you can only keep one kind
of state unless you use multiple buffers.

But let's say you want to add two binary numbers together... then you need 3 states, right?  
The state itself and both digits.

## Idea

This generic turing machine allows one state to do one thing, and then
transition to the state specified in a generic argument. This allows you to use
states almost like functions.

What actually happens behind the scenes is that the generic turing machine copy
pastes your function for each invocation.

For the binary addition example I mentioned, you can now use generic states to
move around on the tape, so you only need to worry about adding one digit at a
time.

## Running

Of course, when implementing a new language as an extension to another it's
easy to cheat and implement stuff that's not really possible to do in any other
language. Therefore, you can choose two modes:

`interactive`: A debugger that allows you to run the turing machine and step
through states, as well as set breakpoints.

`generate`: Generates code for <https://turingmachinesimulator.com/>, proving
that this turing machine isn't doing anything a normal turing machine can't.

Run it like this:
```
generic-turing-machine path/to/input.tm <mode>
```

You may also omit the input file, which makes it default to standard input.

## Syntax

The syntax looks like this:
```
[matching_state] [matching_input] = [output]; [next_state] [movement]
```

`matching_state`: The state name that should be matched against. This can be
generic using `<` and `>`, and all identifiers inside there (separated by `,`)
are placeholders for the state that is passed in the invocation.

`matching_input`: The input being matched. A single digit from 0-9 matches its
ASCII value, and any other character needs to be put inside single quotes, such
as `'a'`. The character _ matches an empty value on the tape.

`output`: Similar to input, but writes the character to the current position on
the tape.

`next_state`: The next state to transition to. Similarly to the matching state,
this can take generic arguments. You can choose to either specify the
placeholder name of a generic argument to the current state, or specify the
name of any state.

`movement`: Either `prev`, `current`, or `next`. After the output has been
written, the buffer is moved relative with this value.

The state name "`start`" is special as it does not take any generic arguments,
matching input, output, or movement, but rather just specifies which state to
start with.

The state name "`finish`" is a pre-defined state that successfully stops the
turing machine.

Example:
```
// Specifies which state to start with
start = skip<invert>

// Generic state that moves the cursor right by one before transitioning to the
// next state
skip<fn> 0 = 0; fn next
skip<fn> 1 = 1; fn next
skip<fn> _ = _; fn next

// State that inverts the binary number until an empty value
invert 0 = 1; invert next
invert 1 = 0; invert next
invert _ = _; finish current
```

Examples can be seen in the `examples/` directory, where you can for example
find the binary addition example I mentioned.
