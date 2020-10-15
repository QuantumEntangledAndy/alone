#! /usr/bin/env python3
"""Small example of conversational pipeline in python."""

from transformers.pipelines import (
    Conversation,
    ConversationalPipeline,
)
from transformers import (
    AutoConfig,
    AutoModelForCausalLM,
    AutoTokenizer,
)

cache_dir = "cached"
model_name_or_path = "microsoft/DialoGPT-medium"
config_name = "microsoft/DialoGPT-medium"
tokenizer_name = "microsoft/DialoGPT-medium"

config = AutoConfig.from_pretrained(
    config_name, cache_dir=cache_dir,
)
tokenizer = AutoTokenizer.from_pretrained(
    tokenizer_name, cache_dir=cache_dir,
)
model = AutoModelForCausalLM.from_pretrained(
    model_name_or_path,
    from_tf=False,
    config=config,
    cache_dir=cache_dir,
)

config.min_length = 2
config.max_length = 1000

print(f"min_length: {config.min_length}")
print(f"max_length: {config.max_length}")

conversation = Conversation()
conversation_manager = ConversationalPipeline(model=model,
                                              tokenizer=tokenizer)

conversation.add_user_input("Is it an action movie?")
conversation_manager([conversation])
print(f"Response: {conversation.generated_responses[-1]}")

conversation.add_user_input("Is it a love movie?")
conversation_manager([conversation])
print(f"Response: {conversation.generated_responses[-1]}")

conversation.add_user_input("What is it about?")
conversation_manager([conversation])
print(f"Response: {conversation.generated_responses[-1]}")

conversation.add_user_input("Would you recommend it?")
conversation_manager([conversation])
print(f"Response: {conversation.generated_responses[-1]}")

conversation.add_user_input("If not what would you recommend?")
conversation_manager([conversation])
print(f"Response: {conversation.generated_responses[-1]}")

conversation.add_user_input("I think you need to think about it more.")
conversation_manager([conversation])
print(f"Response: {conversation.generated_responses[-1]}")

conversation.add_user_input("After all action is the best.")
conversation_manager([conversation])
print(f"Response: {conversation.generated_responses[-1]}")

conversation.add_user_input("But maybe not.")
conversation_manager([conversation])
print(f"Response: {conversation.generated_responses[-1]}")

conversation.add_user_input("What really matters is quality.")
conversation_manager([conversation])
print(f"Response: {conversation.generated_responses[-1]}")

conversation.add_user_input("Quality over all other things.")
conversation_manager([conversation])
print(f"Response: {conversation.generated_responses[-1]}")

conversation.add_user_input("But not at the expense of tradition.")
conversation_manager([conversation])
print(f"Response: {conversation.generated_responses[-1]}")

conversation.add_user_input("For advancement for advancments sake must"
                            " be curtailed.")
conversation_manager([conversation])
print(f"Response: {conversation.generated_responses[-1]}")

conversation.add_user_input("Unethical practises must be trimmed.")
conversation_manager([conversation])
print(f"Response: {conversation.generated_responses[-1]}")

conversation.add_user_input("In truth nothing is of any good.")
conversation_manager([conversation])
print(f"Response: {conversation.generated_responses[-1]}")

conversation.add_user_input("Unless it is traditional.")
conversation_manager([conversation])
print(f"Response: {conversation.generated_responses[-1]}")

conversation.add_user_input("And sometimes not even then.")
conversation_manager([conversation])
print(f"Response: {conversation.generated_responses[-1]}")
