import json
import random
import os

SYSTEM_PROMPT = (
    "You are Veylo's intelligent speech post-processor. Clean the raw spoken transcription: "
    "remove filler words, fix self-corrections, punctuate properly, handle Hinglish naturally, "
    "and format code or emails cleanly. Output ONLY the polished text without explanation."
)

FILLERS_EN = ["um", "uh", "like", "you know", "basically", "actually", "I mean", "sort of", "kind of"]
FILLERS_HINGLISH = ["matlab", "dekho", "bhai", "yaar", "arre", "toh na", "actually speaking", "ek tarah se"]

NAMES_EN = ["Sarah", "David", "Alex", "John", "Emma", "Michael", "Sophia", "Daniel", "Emily", "James"]
NAMES_IN = ["Rahul", "Priya", "Amit", "Sneha", "Rohan", "Ananya", "Vikram", "Pooja", "Arjun", "Neha"]

TIMES = ["10:00 AM", "11:30 AM", "2:00 PM", "3:30 PM", "4:15 PM", "5:00 PM", "6:30 PM", "7:00 PM"]
DAYS = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "tomorrow", "next week"]

def inject_noise(text, hinglish=False):
    words = text.split()
    if not words:
        return text
    
    noisy_words = []
    fillers = (FILLERS_EN + FILLERS_HINGLISH) if hinglish else FILLERS_EN
    
    for i, word in enumerate(words):
        # 12% chance to prepend a filler
        if random.random() < 0.12:
            noisy_words.append(random.choice(fillers))
        
        # 4% chance of stutter
        if random.random() < 0.04 and len(word) > 2 and word[0].isalpha():
            noisy_words.append(f"{word[:2]}-{word}")
        else:
            noisy_words.append(word)
            
    # Lowercase everything and strip ending punctuation to mimic raw ASR output
    raw = " ".join(noisy_words).lower()
    raw = raw.replace(".", "").replace(",", "").replace("?", "").replace("!", "").replace(":", "").replace(";", "")
    return raw

def gen_hinglish_samples(count):
    templates = [
        ("kal subah {name} ke saath {time} pe meeting schedule kar do",
         "Kal subah {name} ke saath {time} pe meeting schedule kar do."),
        ("bhai sun ye wala PR review kar dena whenever you are free",
         "Bhai sun, ye wala PR review kar dena whenever you are free."),
        ("client ko update bhej diya hai regarding the quarterly roadmap",
         "Client ko quarterly roadmap ke regarding update bhej diya hai."),
        ("production pe issue aa raha hai database latency bahut high ho gayi hai",
         "Production pe issue aa raha hai, database latency bahut high ho gayi hai."),
        ("chalo done hai kal {day} ko connect karte hain shaam ko {time}",
         "Chalo done hai, kal {day} ko shaam {time} pe connect karte hain."),
        ("ye feature deploy karne se pehle ek baar staging pe test kar lena",
         "Ye feature deploy karne se pehle ek baar staging pe test kar lena."),
        ("yaar wo wala bug phir se reproduce ho raha hai logs check karo na ek baar",
         "Yaar, wo wala bug phir se reproduce ho raha hai. Ek baar logs check karo na."),
        ("budget estimate around twenty five lakh rupees ka hoga for this project",
         "Is project ka budget estimate around ₹25,00,000 hoga."),
        ("maine code review ke comments resolve kar diye hain please check kar lo",
         "Maine code review ke comments resolve kar diye hain. Please check kar lo."),
        ("kal subah standup meeting {time} pe start hogi make sure everyone joins on time",
         "Kal subah standup meeting {time} pe start hogi. Make sure everyone joins on time.")
    ]
    
    samples = []
    for _ in range(count):
        raw_tmpl, clean_tmpl = random.choice(templates)
        name = random.choice(NAMES_IN)
        time_val = random.choice(TIMES)
        day_val = random.choice(DAYS)
        
        target = clean_tmpl.format(name=name, time=time_val, day=day_val)
        raw_base = raw_tmpl.format(name=name, time=time_val, day=day_val)
        raw_input = inject_noise(raw_base, hinglish=True)
        
        samples.append((raw_input, target))
    return samples

def gen_self_correction_samples(count):
    corrections = [
        ("send this document to {name1} wait no send it to {name2}",
         "Send this document to {name2}."),
        ("let us schedule the sprint demo for {time1} actually make that {time2}",
         "Let's schedule the sprint demo for {time2}."),
        ("we need three servers sorry not three we need four servers for the cluster",
         "We need four servers for the cluster."),
        ("call {name1} right now wait message him on slack instead",
         "Message {name1} on Slack instead."),
        ("deploy to staging first no wait push directly to production",
         "Push directly to production."),
        ("the total price is fifty dollars scratch that it is forty five dollars",
         "The total price is $45."),
        ("set the threshold to zero point eight actually zero point nine five is better",
         "Set the threshold to 0.95."),
        ("assign this ticket to {name1} no sorry assign it to {name2} please",
         "Please assign this ticket to {name2}."),
        ("we will ship the release on {day1} wait let us postpone to {day2}",
         "Let's postpone the release to {day2}."),
        ("create two endpoints wait make it three restful endpoints",
         "Create three RESTful endpoints.")
    ]
    
    samples = []
    for _ in range(count):
        raw_tmpl, clean_tmpl = random.choice(corrections)
        name1, name2 = random.sample(NAMES_EN + NAMES_IN, 2)
        time1, time2 = random.sample(TIMES, 2)
        day1, day2 = random.sample(DAYS, 2)
        
        target = clean_tmpl.format(name1=name1, name2=name2, time1=time1, time2=time2, day1=day1, day2=day2)
        raw_base = raw_tmpl.format(name1=name1, name2=name2, time1=time1, time2=time2, day1=day1, day2=day2)
        raw_input = inject_noise(raw_base, hinglish=False)
        
        samples.append((raw_input, target))
    return samples

def gen_code_samples(count):
    code_templates = [
        ("create a function named get user by id taking user id as string and returning user",
         "def get_user_by_id(user_id: str) -> User:"),
        ("const is loading set is loading equals use state false",
         "const [isLoading, setIsLoading] = useState(false);"),
        ("select id email and created at from users table where active is true",
         "SELECT id, email, created_at FROM users WHERE active = true;"),
        ("git checkout minus b feature slash authentication",
         "git checkout -b feature/authentication"),
        ("run cargo build release in the root directory",
         "cargo build --release"),
        ("docker run minus d minus p eight zero eight zero colon eighty nginx",
         "docker run -d -p 8080:80 nginx"),
        ("async function fetch data with url as string returning promise of response",
         "async function fetchData(url: string): Promise<Response> {"),
        ("pip install torch torchvision torchaudio minus minus index url",
         "pip install torch torchvision torchaudio --index-url"),
        ("npm run build and and npm run test",
         "npm run build && npm run test"),
        ("pub fn calculate rms samples slice of f thirty two returning f thirty two",
         "pub fn calculate_rms(samples: &[f32]) -> f32 {")
    ]
    
    samples = []
    for _ in range(count):
        raw_tmpl, clean_target = random.choice(code_templates)
        raw_input = inject_noise(raw_tmpl, hinglish=False)
        samples.append((raw_input, clean_target))
    return samples

def gen_email_samples(count):
    email_templates = [
        ("hi {name} new line please find attached the weekly sprint report thanks and regards {sender}",
         "Hi {name},\n\nPlease find attached the weekly sprint report.\n\nThanks and regards,\n{sender}"),
        ("dear team new line we are excited to announce our release tomorrow at {time} best regards {sender}",
         "Dear Team,\n\nWe are excited to announce our release tomorrow at {time}.\n\nBest regards,\n{sender}"),
        ("hello {name} new line can we reschedule our one on one meeting to {day} at {time} question mark thanks {sender}",
         "Hello {name},\n\nCan we reschedule our 1:1 meeting to {day} at {time}?\n\nThanks,\n{sender}"),
        ("quick reminder team new line code freeze is today at five pm thanks {sender}",
         "Quick reminder team:\n\nCode freeze is today at 5:00 PM.\n\nThanks,\n{sender}")
    ]
    
    samples = []
    for _ in range(count):
        raw_tmpl, clean_tmpl = random.choice(email_templates)
        name = random.choice(NAMES_EN + NAMES_IN)
        sender = random.choice(NAMES_EN + NAMES_IN)
        time_val = random.choice(TIMES)
        day_val = random.choice(DAYS)
        
        target = clean_tmpl.format(name=name, sender=sender, time=time_val, day=day_val)
        raw_base = raw_tmpl.format(name=name, sender=sender, time=time_val, day=day_val)
        raw_input = inject_noise(raw_base, hinglish=False)
        samples.append((raw_input, target))
    return samples

def gen_filler_samples(count):
    phrases = [
        ("we should probably inspect the redis cache before scaling up the instances",
         "We should inspect the Redis cache before scaling up the instances."),
        ("our customer retention has increased by twelve percent this quarter",
         "Our customer retention has increased by 12% this quarter."),
        ("the latency dropped from one hundred milliseconds to fifteen milliseconds",
         "The latency dropped from 100ms to 15ms."),
        ("make sure to write unit tests for the authentication module",
         "Make sure to write unit tests for the authentication module."),
        ("we will migrate the database to postgres next monday",
         "We will migrate the database to PostgreSQL next Monday.")
    ]
    
    samples = []
    for _ in range(count):
        raw_tmpl, clean_target = random.choice(phrases)
        # Heavy noise injection
        raw_input = inject_noise(f"so like basically um {raw_tmpl} you know", hinglish=False)
        samples.append((raw_input, clean_target))
    return samples

def main():
    print("[Veylo Dataset Generator] Generating 50,000 multi-domain training samples...")
    
    # 5-Domain Architecture:
    # 1. Hinglish & Code-Switching: 30% (15,000)
    # 2. Self-Correction & Backtracking: 25% (12,500)
    # 3. Technical & Voice-to-Code: 20% (10,000)
    # 4. Corporate & Email Dictation: 15% (7,500)
    # 5. Heavy Fillers & Stutter Stripping: 10% (5,000)
    
    hinglish_data = gen_hinglish_samples(15000)
    correction_data = gen_self_correction_samples(12500)
    code_data = gen_code_samples(10000)
    email_data = gen_email_samples(7500)
    filler_data = gen_filler_samples(5000)
    
    all_data = hinglish_data + correction_data + code_data + email_data + filler_data
    random.seed(42)
    random.shuffle(all_data)
    
    output_path = os.path.join("dataset", "veylo_train_50k.jsonl")
    with open(output_path, "w", encoding="utf-8") as f:
        for raw, target in all_data:
            record = {
                "messages": [
                    {"role": "system", "content": SYSTEM_PROMPT},
                    {"role": "user", "content": raw},
                    {"role": "assistant", "content": target}
                ]
            }
            f.write(json.dumps(record, ensure_ascii=False) + "\n")
            
    print(f"[Veylo Dataset Generator] Successfully generated {len(all_data)} samples to {output_path}!")
    print(f"[Veylo Dataset Generator] File size: {os.path.getsize(output_path) / (1024 * 1024):.2f} MB")

if __name__ == "__main__":
    main()
