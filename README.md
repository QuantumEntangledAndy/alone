Alone
---


This is a simple chatbot in rust. I made it for fun so don't expect any
docs.

It has a telegram part too, which was fun to add in.

OK so in terms of the config tomls.

It expects a config toml in the cwd called alone.toml with this:

```toml
model_name = "NameOfModel"
max_context = 6 # How much context to keep in memory

telegram_token = "TOKEN" # Optional: Telegram token
telegram_id = 123456 # Optional: ID of user to chat to
```

The `model_name` will be used the name where the files required
for the bot. If set leave it as ``"default"`` it will pull down
DiagloGPT from huggingface's repository.

If `model_name` is not blank it will look for a folder in
the cwd that has these files

```
model_name.model/
    config.json
    model.ot
    vocab.json
    merges.txt
```

To use telegram both `telegram_token` and `telegram_id` must be set.
The bot will only chat with a user with the given `telegram_id`.

If either `telegram_token` or `telegram_id` are not set it defaults to console input.

There is also an optional `wordimages.toml` that can be placed in
the cwd.  It expects to contain.


```toml
[[word_images]]
path = "./wordimages/life.jpg"
words = [ "nature", "life" ]


[[word_images]]
path = "./wordimages/friends.jpg"
words = [ "friends", "together" ]
```

The words will be added to a zero shot classification model. If any
dialogue from the bot has `>0.96` score it will randomly select one
of the images that match and then display that image. For the image
to work on the console you need to have the `imgcat` program
installed.
