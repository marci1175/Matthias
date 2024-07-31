---
description: >-
  This page helps you understand getter functions in the context of Matthias
  scripting.
---

# 🎯 Getter functions

### Getters are functions that are used to access properties on an object. In this case, the properties / fields of the Application struct and its children.

Getter functions in Matthias return the serialized (Serialized with [Serde](https://serde.rs/)) value of the given entry.

#### Script example for printing out the logged in User's username

```lua
print(userinformation.username)
```

Please note that when accessing struct, all names are in lowercase compared to the source code.

```rust
struct UserInformation {
    hello: String
}
```

Translates to:

```lua
userinformation.hello
```
