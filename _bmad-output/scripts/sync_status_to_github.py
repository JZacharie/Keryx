import os
import yaml
import requests
import json
import argparse
import re

# Configuration
GITHUB_API_URL = "https://api.github.com"

STATUS_LABELS = {
    "backlog": "status: backlog",
    "in-progress": "status: in-progress",
    "review": "status: review",
    "done": "status: done"
}

def get_issue_labels(repo, token, issue_number):
    url = f"{GITHUB_API_URL}/repos/{repo}/issues/{issue_number}"
    headers = {
        "Authorization": f"token {token}",
        "Accept": "application/vnd.github.v3+json"
    }
    response = requests.get(url, headers=headers)
    if response.status_code == 200:
        return [l['name'] for l in response.json().get('labels', [])]
    return []

def update_github_issue(repo, token, issue_number, status):
    url = f"{GITHUB_API_URL}/repos/{repo}/issues/{issue_number}"
    headers = {
        "Authorization": f"token {token}",
        "Accept": "application/vnd.github.v3+json"
    }
    
    current_labels = get_issue_labels(repo, token, issue_number)
    
    # Remove old status labels
    new_labels = [l for l in current_labels if not l.startswith("status: ")]
    if status in STATUS_LABELS:
        new_labels.append(STATUS_LABELS[status])
    
    data = {
        "labels": new_labels,
        "state": "closed" if status == "done" else "open"
    }
    
    response = requests.patch(url, headers=headers, json=data)
    if response.status_code == 200:
        return response.json()
    else:
        print(f"Error updating issue #{issue_number}: {response.status_code} - {response.text}")
        return None

def main():
    parser = argparse.ArgumentParser(description="Sync Keryx Story Status to GitHub Issues")
    parser.add_argument("--repo", required=True, help="GitHub repository (e.g., JZacharie/Keryx)")
    parser.add_argument("--token", required=True, help="GitHub Personal Access Token")
    parser.add_argument("--status-file", default="_bmad-output/implementation-artifacts/sprint-status.yaml", help="Path to sprint-status.yaml")
    parser.add_argument("--mapping-file", default="_bmad-output/planning-artifacts/github_mapping.json", help="Path to github_mapping.json")
    
    args = parser.parse_args()

    if not os.path.exists(args.status_file):
        print(f"Status file not found: {args.status_file}")
        return

    if not os.path.exists(args.mapping_file):
        print(f"Mapping file not found: {args.mapping_file}")
        return

    with open(args.status_file, "r") as f:
        status_data = yaml.safe_load(f)

    with open(args.mapping_file, "r") as f:
        mapping = json.load(f)

    dev_status = status_data.get("development_status", {})
    
    print(f"Syncing status to GitHub repository: {args.repo}...")

    for key, status in dev_status.items():
        # Match story key: e.g., "1-1-project-skeleton..." -> "1.1"
        match = re.match(r"(\d+)-(\d+)-", key)
        if match:
            story_id = f"{match.group(1)}.{match.group(2)}"
            
            if story_id in mapping:
                github_id = mapping[story_id]["github_id"]
                print(f"  Story {story_id} ({status}) -> Issue #{github_id}")
                update_github_issue(args.repo, args.token, github_id, status)
            else:
                print(f"  Story {story_id} ({status}) -> No GitHub mapping found. Skipping.")

    print("\nStatus sync complete.")

if __name__ == "__main__":
    main()
