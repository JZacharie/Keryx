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

    # Regex to find Epics
    epic_pattern = r"## Epic (\d+): (.*?)\n(.*?)(?=\n## Epic|\n#|\Z)"
    epics = re.findall(epic_pattern, content, re.DOTALL)

    parsed_data = []

    for epic_num, epic_title, epic_content in epics:
        # Regex to find Stories within an Epic
        story_pattern = r"### Story (\d+\.\d+): (.*?)\n(.*?)(?=\n### Story|\Z)"
        stories = re.findall(story_pattern, epic_content, re.DOTALL)
        
        epic_data = {
            "number": epic_num,
            "title": epic_title.strip(),
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

    if not os.path.exists(args.file):
        print(f"File not found: {args.file}")
        return

    print(f"Reading {args.file}...")
    epics = parse_epics(args.file)
    
    mapping = {}

    for epic in epics:
        print(f"\nProcessing Epic {epic['number']}: {epic['title']}")
        epic_label = f"Epic: {epic['title']}"
        
        for story in epic['stories']:
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
    with open("_bmad-output/planning-artifacts/github_mapping.json", "w") as f:
        json.dump(mapping, f, indent=2)
    print(f"\nSync complete. Mapping saved to _bmad-output/planning-artifacts/github_mapping.json")

if __name__ == "__main__":
    main()
