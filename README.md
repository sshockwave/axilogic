# axilogic

## Meta Operations
```bash
# Stack Operations
push <n> # Push stack[-n] to top
swap # Swap the two upper most elements on the stack
pop # Pop element from stack

# Functions
symbol # Push a new symbol
forall # collapse a symbol and an expression to a function (for all expression)
apply # take a forall predicate (-2) and an expression (-1) to replace all occurences

# Deduction
express # Enter falsy mode which allows non-truth values
assume # Make the top element (-1) a antecedent (only in falsy mode). If the top element is the last falsy value, enter truthy mode.
abstract # Collapse an assumption (-2) and an expression (-1) to a predicate (-1)
apply # modus ponens. Apply antecedent (-1) to predicate (-2), and get its consequent (-1). (only in truthy mode)
trust # replace predicate (-1) with its consequent (-1) (only in falsy mode)

# Concept
export <name> # Export the stack top (-1) as a function that receives the current stack
concept <name> # Like `export`, but is a closure and no stack object is needed
refer <name> # Push a reference representing the exported function or concept
unbind # Unbind concept (-1) to antecedent (0) and consequent (-1)
```
