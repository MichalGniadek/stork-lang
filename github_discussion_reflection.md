It would be nice if bevy supported richer method/function reflection

### Use cases

- Obvious one: scripting languages. Being able to call functions/methods on types would allow for richer interaction and less code duplication between rust and scripting languages
- Editor: being able to annotate some function with something like `#[reflect(@ExposeInEditor)]` could show a button in component inspector
  - [Unity addon NaughtyInspector allows something similar](https://github.com/dbrizov/NaughtyAttributes?tab=readme-ov-file#button)
  - [Godot is adding something similar in 4.4](https://godotengine.org/article/dev-snapshot-godot-4-4-dev-3/#export_tool_button-annotation)

### Requirements

- Being able to automatically generate a struct with reflected methods from `impl` and `impl Trait` items
  - Being able to ignore certain methods, rename them or add custom attributes
- Being able to iterate over traits implemented by a type
- Being able to iterate over all impls blocks implemented for a type
- Being able to iterate over all methods and constants in an `impl`, `impl Trait` block
- Being able to iterate over all functions with proper type path (modules and not just names)

### Proposal

Introduce new traits: `Impl` and `TraitImpl` which would have methods for iterating over `DynamicFunction` representations of methods inside of the impl blocks.`TraitImpl` would also have methods for existing "cast `dyn Reflect` to `dyn Trait`" functionality. If the trait isn't object safe they would return an error.

Also introduce `ImplInfo` and `TraitImplInfo` e.g. docs information and custom attributes and add doc and attributes information to `FunctionInfo`.

### Questions

- How to handle existing non-stands `ReflectTrait` e.g. `ReflectComponent`?
- Not sure what bevy stance would be on annotating a lot of `impl Type` and `impl Trait on Type` with `#[reflect]`. It's more intrusive then a derive but probably on the same level as `#[reflect(Trait)]` that already needs to be added for types?
  - e.g. methods on [Time](https://docs.rs/bevy/latest/bevy/time/struct.Time.html) would be very useful for scripting languages

### Additional notes

- Already some discussion in #8228
- If #15030 is merged we could probably use the same mechanism for automatic type data registration for each type (at least for some cases)
