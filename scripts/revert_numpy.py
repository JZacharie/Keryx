import os

def revert_numpy(file_path):
    with open(file_path, 'r') as f:
        lines = f.readlines()
    
    new_lines = []
    changed = False
    for line in lines:
        if line.startswith('numpy=='):
            new_line = 'numpy==2.2.6\n'
            if new_line != line:
                changed = True
            new_lines.append(new_line)
        else:
            new_lines.append(line)
    
    if changed:
        with open(file_path, 'w') as f:
            f.writelines(new_lines)
        print(f"Reverted numpy in {file_path}")

for root, dirs, files in os.walk('/home/joseph/git/Keryx/services'):
    if 'requirements.txt' in files:
        revert_numpy(os.path.join(root, 'requirements.txt'))
