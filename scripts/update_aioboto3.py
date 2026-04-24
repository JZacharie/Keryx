import os

def update_aioboto3(file_path):
    with open(file_path, 'r') as f:
        lines = f.readlines()
    
    new_lines = []
    changed = False
    for line in lines:
        if line.strip() == 'aioboto3' or line.startswith('aioboto3=='):
            new_line = 'aioboto3==13.4.0\n'
            if new_line != line:
                changed = True
            new_lines.append(new_line)
        else:
            new_lines.append(line)
    
    if changed:
        with open(file_path, 'w') as f:
            f.writelines(new_lines)
        print(f"Updated aioboto3 in {file_path}")

for root, dirs, files in os.walk('/home/joseph/git/Keryx/services'):
    if 'requirements.txt' in files:
        update_aioboto3(os.path.join(root, 'requirements.txt'))
