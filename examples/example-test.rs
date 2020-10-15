extern crate anyhow;

use rust_bert::pipelines::conversation::{ConversationManager, ConversationModel};

fn main() -> anyhow::Result<()> {
    let conversation_model = ConversationModel::new(Default::default())?;
    let mut conversation_manager = ConversationManager::new();

    let conversation_1_id =
        conversation_manager.create("Going to the movies tonight - any suggestions?");

    let output = conversation_model.generate_responses(&mut conversation_manager);

    println!("{:?}", output);

    let _ = conversation_manager
        .get(&conversation_1_id)
        .unwrap()
        .add_user_input("Is it an action movie?");

    let output = conversation_model.generate_responses(&mut conversation_manager);

    println!("{:?}", output);

    let _ = conversation_manager
        .get(&conversation_1_id)
        .unwrap()
        .add_user_input("Is it a love movie?");

    let output = conversation_model.generate_responses(&mut conversation_manager);

    println!("{:?}", output);

    let _ = conversation_manager
        .get(&conversation_1_id)
        .unwrap()
        .add_user_input("What is it about?");

    let output = conversation_model.generate_responses(&mut conversation_manager);

    println!("{:?}", output);

    let _ = conversation_manager
        .get(&conversation_1_id)
        .unwrap()
        .add_user_input("Would you recommend it?");

    let output = conversation_model.generate_responses(&mut conversation_manager);

    println!("{:?}", output);

    let output = conversation_model.generate_responses(&mut conversation_manager);

    println!("{:?}", output);

    let _ = conversation_manager
        .get(&conversation_1_id)
        .unwrap()
        .add_user_input("If not what would you recommend?");

    let output = conversation_model.generate_responses(&mut conversation_manager);
    println!("{:?}", output);

    let _ = conversation_manager
        .get(&conversation_1_id)
        .unwrap()
        .add_user_input("I think you need to think about it more.");

    let output = conversation_model.generate_responses(&mut conversation_manager);
    println!("{:?}", output);

    let _ = conversation_manager
        .get(&conversation_1_id)
        .unwrap()
        .add_user_input("After all action is the best.");

    let output = conversation_model.generate_responses(&mut conversation_manager);
    println!("{:?}", output);

    let _ = conversation_manager
        .get(&conversation_1_id)
        .unwrap()
        .add_user_input("But maybe not.");

    let output = conversation_model.generate_responses(&mut conversation_manager);
    println!("{:?}", output);

    let _ = conversation_manager
        .get(&conversation_1_id)
        .unwrap()
        .add_user_input("What really matters is quality.");

    let output = conversation_model.generate_responses(&mut conversation_manager);
    println!("{:?}", output);

    let _ = conversation_manager
        .get(&conversation_1_id)
        .unwrap()
        .add_user_input("Quality over all other things.");

    let output = conversation_model.generate_responses(&mut conversation_manager);
    println!("{:?}", output);

    let _ = conversation_manager
        .get(&conversation_1_id)
        .unwrap()
        .add_user_input("But not at the expense of tradition.");

    let output = conversation_model.generate_responses(&mut conversation_manager);
    println!("{:?}", output);

    let _ = conversation_manager
        .get(&conversation_1_id)
        .unwrap()
        .add_user_input("For advancement for advancments sake must be curtailed.");

    let output = conversation_model.generate_responses(&mut conversation_manager);
    println!("{:?}", output);

    let _ = conversation_manager
        .get(&conversation_1_id)
        .unwrap()
        .add_user_input("Unethical practises must be trimmed.");

    let output = conversation_model.generate_responses(&mut conversation_manager);
    println!("{:?}", output);

    let _ = conversation_manager
        .get(&conversation_1_id)
        .unwrap()
        .add_user_input("In truth nothing is of any good.");

    let output = conversation_model.generate_responses(&mut conversation_manager);
    println!("{:?}", output);

    let _ = conversation_manager
        .get(&conversation_1_id)
        .unwrap()
        .add_user_input("Unless it is traditional.");

    let output = conversation_model.generate_responses(&mut conversation_manager);
    println!("{:?}", output);

    let _ = conversation_manager
        .get(&conversation_1_id)
        .unwrap()
        .add_user_input("And sometimes not even then.");

    let output = conversation_model.generate_responses(&mut conversation_manager);
    println!("{:?}", output);

    Ok(())
}
