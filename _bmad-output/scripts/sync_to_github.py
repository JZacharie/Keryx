import os
import re
import requests
import json
import argparse

# Configuration
GITHUB_API_URL = "https://api.github.com"

def parse_epics(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    # Improved Regex for Epics (handles optional spaces and newlines)
    epic_pattern = r"## Epic (\d+):\s*(.*?)\n"
    epics = list(re.finditer(epic_pattern, content))

    parsed_data = []

    for i, match in enumerate(epics):
        epic_num = match.group(1)
        epic_title = match.group(2).strip()
        
        # Get content between this epic and the next one
        start_pos = match.end()
        end_pos = epics[i+1].start() if i + 1 < len(epics) else len(content)
        epic_content = content[start_pos:end_pos]

        # Improved Regex for Stories (handles the '### Story X.Y: Title' format)
        story_pattern = r"### Story (\d+\.\d+):\s*(.*?)\n(.*?)(?=\n### Story|\n## Epic|\n#|\Z)"
        stories = re.findall(story_pattern, epic_content, re.DOTALL)
        
        epic_data = {
            "number": epic_num,
            "title": epic_title,
            "stories": []
        }
        
        for story_num, story_title, story_body in stories:
            epic_data["stories"].append({
                "number": story_num,
                "title": story_title.strip(),
                "body": story_body.strip()
            })
            
        parsed_data.append(epic_data)
        
    return parsed_data

def create_github_issue(repo, token, title, body, labels):
    url = f"{GITHUB_API_URL}/repos/{repo}/issues"
    headers = {
        "Authorization": f"token {token}",
        "Accept": "application/vnd.github.v3+json"
    }
    data = {
        "title": title,
        "body": body,
        "labels": labels
    }
    response = requests.post(url, headers=headers, json=data)
    if response.status_code == 201:
        return response.json()
    else:
        print(f"Error creating issue: {response.status_code} - {response.text}")
        return None

def main():
    parser = argparse.ArgumentParser(description="Sync Keryx Epics and Stories to GitHub Issues")
    parser.add_argument("--repo", required=True, help="GitHub repository (e.g., JZacharie/Keryx)")
    parser.add_argument("--token", required=True, help="GitHub Personal Access Token")
    parser.add_argument("--file", default="_bmad-output/planning-artifacts/epics.md", help="Path to epics.md")
    
    args = parser.parse_args()

    mapping_file = "_bmad-output/planning-artifacts/github_mapping.json"
    mapping = {}
    if os.path.exists(mapping_file):
        with open(mapping_file, "r") as f:
            mapping = json.load(f)

    if not os.path.exists(args.file):
        print(f"File not found: {args.file}")
        return

    print(f"Reading {args.file}...")
    epics = parse_epics(args.file)
    
    for epic in epics:
        print(f"\nProcessing Epic {epic['number']}: {epic['title']}")
        # Clean label name: remove commas and special chars
        clean_title = epic['title'].replace(",", "").replace("&", "and")
        epic_label = f"Epic: {clean_title}"
        
        for story in epic['stories']:
            if story['number'] in mapping:
                print(f"  Story {story['number']} already exists (Issue #{mapping[story['number']]['github_id']}). Skipping.")
                continue

            title = f"[{story['number']}] {story['title']}"
            body = f"## {story['title']}\n\n{story['body']}\n\n---\n*Part of Epic {epic['number']}: {epic['title']}*"
            
            print(f"  Creating Issue for Story {story['number']}...")
            issue = create_github_issue(args.repo, args.token, title, body, [epic_label, "story"])
            
            if issue:
                mapping[story['number']] = {
                    "github_id": issue['number'],
                    "url": issue['html_url']
                }
                print(f"    Success: {issue['html_url']}")

    # Save mapping
    with open(mapping_file, "w") as f:
        json.dump(mapping, f, indent=2)
    print(f"\nSync complete. Mapping updated in {mapping_file}")

if __name__ == "__main__":
    main()
