import sys
from status import status

def main():
    command = sys.argv[1] if len(sys.argv) > 1 else '--help'
    if command == '--help':
        sys.stdout.write('status-cli base')
    elif command == 'base':
        sys.stdout.write(status())
    else:
        sys.stderr.write('unknown command\n')
        return 2
    return 0

if __name__ == '__main__':
    raise SystemExit(main())
