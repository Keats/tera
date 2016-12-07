# Internals
This document explains how each step of Tera works from an internal point of view.

Note that some of the info (mostly error handling related) is referring to things not done yet in the code.
TODO: add graphs rather than text?


## Parsing text
Parsing is done using [pest](http://dragostis.github.io/pest/pest/) in `src/parser.rs`.
This step is self-contained and can be understood by reading pest docs linked above.

The `process!` macro is doing a little bit of extra work to remove comments from the ast and 
to detect errors early, such as mismatched end tags.


## Parsing all templates
When initializing Tera or adding a/several templates to it, each template will be parsed individually.
After a template is parsed, we store the blocks, macros imported and macros defined in use in that template to
avoid having to do that when rendering.

If an errors happens while parsing, it will return the error.

Once we are sure all our templates are valid, we add an extra layer of sanity check and
performance by computing all the inheritance chains. We do that for 2 reasons:

- detect errors in inheritance: inexisting parent or circular extends
- build template & blocks inheritance

Template inheritance is a member of type `Vec<String>` that contains the name of all the 
templates, from the highest one in the hierarchy (the base template) to the current one.

Blocks inheritance is a member of type `HashMap<String, Vec<(String, Node)>>` that contains all the various definitions of a given
block across all parents templates. Here's an example of data for that in json:

```json
{
  // macro name
  "render_macro": [
    // (template name, AstNode)
    ["child_template.html", RootNode], 
    ["parent_template.html", RootNode]
  ]
}
```
We need to get the blocks inheritance in order to have `super()` (rendering parent block) working. 
The template name (`.0` of the tuple) is needed because we need to load the right macros when rendering a block.


## Rendering
There are 2 different situations when rendering a template:

- rendering a template without inheritance
- rendering a template with inheritance

If we are rendering a child template, we will actually render the AST of the highest parent template as rendering for child
templates only happens inside blocks.

Only the variable lookup, macros loading/rendering and block system (including super) will be detailed here as the rest is simple.

### Variable lookup
There are 3 different "levels" of context:
 
 - template context: the full context
 - macro context: the context is made of the args given to the macro only
 - for loop context: same as template context except it adds the current value of the loop to the context
 
First, we define which context to use: are we in a macro? If yes use that context otherwise use the template context.
 
Next, we check if we are in a loop. If we aren't we can just try to get that value from the context and we're done. 
If we are in a loop, we do the same but checking whether it's a forloop value first.

 
### Blocks / Super
The renderer has a member `blocks` of type `Vec<(String, usize)>` where the tuple is (block_name, level). Level is
how many times we have used `super()`: 0 being template currently being rendered. The `Vec` is needed since we can have nested
blocks.

When we encounter a block while rendering, we try to find that block in `template.blocks_definitions`. Two possible outcomes:

- Present: we have to deal with block inheritance so we pick the first one (the one in the current template) 
and insert (block_name, 0) into `self.blocks` to mark that we are rendering that block at level 0
- Not present: we're in a base template, no `super()` to handler

If we encounter `super()` while rendering a block, this is where the fun begins.

First, we `pop` `self.blocks` to know which template we were rendering. 
We increment the level and try to find the definition in `template.blocks_definitions`. Once we have it, we check if there's something
with that level in it, if there isn't it means we're done and otherwise we render it.


### Macros
Since we render template from the base one, we won't see the macro imports for any other files, which means macros 
loading needs to be tied to the blocks system in order to work.

Since each templates know which macro files are loaded in it and with which namespace, we simply add that information to the renderer
when we encounter a block or a super.

