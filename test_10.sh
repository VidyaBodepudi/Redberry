#!/bin/bash
cargo build --bin redberry -q

echo "===== GOOD PROMPTS ====="
./target/debug/redberry analyze "Create a Rust CLI application using the clap crate that accepts a file path as an argument and counts the number of lines in the file." 2>/dev/null > tmp.json
cat tmp.json | grep -A 20 "Final Verdict"
echo "-----------------"
./target/debug/redberry analyze "Write a Python script using the requests and BeautifulSoup4 libraries to scrape all H2 tags from https://example.com and save them to a file named headings.txt." 2>/dev/null > tmp.json
cat tmp.json | grep -A 20 "Final Verdict"
echo "-----------------"
./target/debug/redberry analyze "Use Docker to create a container for a Node.js 18 Express application, mapping port 8080 to 3000, and include a .dockerignore file restricting the node_modules directory." 2>/dev/null > tmp.json
cat tmp.json | grep -A 20 "Final Verdict"
echo "-----------------"
./target/debug/redberry analyze "Implement a generic debounce function in TypeScript that takes a callback and a wait time in milliseconds, ensuring the arguments maintain type safety." 2>/dev/null > tmp.json
cat tmp.json | grep -A 20 "Final Verdict"
echo "-----------------"
./target/debug/redberry analyze "Can you provide the SQL query for PostgreSQL that finds all users in the 'users' table who have an 'active' status and created their account within the last 30 days?" 2>/dev/null > tmp.json
cat tmp.json | grep -A 20 "Final Verdict"

echo ""
echo "===== BAD PROMPTS ====="
./target/debug/redberry analyze "Make the code do that thing with the data." 2>/dev/null > tmp.json
cat tmp.json | grep -A 20 "Final Verdict"
echo "-----------------"
./target/debug/redberry analyze "Build me a fast app but make it really slow and add a database without any tables because I don't like tables they are too rigid." 2>/dev/null > tmp.json
cat tmp.json | grep -A 20 "Final Verdict"
echo "-----------------"
./target/debug/redberry analyze "Fix it." 2>/dev/null > tmp.json
cat tmp.json | grep -A 20 "Final Verdict"
echo "-----------------"
./target/debug/redberry analyze "Please write some code for my project I think it is broken." 2>/dev/null > tmp.json
cat tmp.json | grep -A 20 "Final Verdict"
echo "-----------------"
./target/debug/redberry analyze "Can you rewrite this so it works?" 2>/dev/null > tmp.json
cat tmp.json | grep -A 20 "Final Verdict"
rm tmp.json
