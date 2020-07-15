#!/usr/bin/python
from sys import argv
from os import getcwd
from subprocess import call

import re
import json

# execute from fitm/
def main():
    open_fds = ""

    lines = [x.strip("\n") for x in open("./restore.sh.tmp", "r").readlines()]
    if argv[1]:
        base_state = f"{getcwd()}/active-state/{argv[1]}"[1:]
        cur_state = f"{getcwd()}/active-state/{argv[2]}"[1:]

        lines.append(f"    --inherit-fd \"fd[1]:{base_state}/stdout\" \\")
        lines.append(f"    --inherit-fd \"fd[2]:{base_state}/stderr\" \\")

        call(f"crit decode -i /{base_state}/snapshot/files.img --pretty -o ./file".split())
        call(f"crit decode -i /{base_state}/snapshot/fdinfo-2.img --pretty -o ./fdinfo".split())

        file_info = json.load(open("./file", "r"))
        fd_info = json.load(open("./fdinfo", "r"))

        files = filter(lambda x: "reg" in x.keys() and "/fd/" in x["reg"]["name"], file_info["entries"])
        files = map(lambda x: (x["id"], x["reg"]["name"]), files)

        mapping = []

        for f in files:
            fd = list(filter(lambda x: x["id"] == f[0], fd_info["entries"]))[0]
            mapping.append((fd["fd"], f[1]))

        open_fds += 'if [[ -z "${LETS_DO_THE_TIMEWARP_AGAIN}" ]]; then\n'
        open_fds += f"  exec 1> /{cur_state}/stdout\n"
        open_fds += f"  exec 2> /{cur_state}/stderr\n"
        open_fds += "  echo 'Using stdout/sterr'\n"
        open_fds += "else\n"
        open_fds += "  exec 1> /dev/null\n"
        open_fds += "  exec 2> /dev/null\n"
        open_fds += "  echo 'Using /dev/null'\n"
        open_fds += "fi\n\n"

        for m in mapping:
            open_fds += f"exec {m[0]}< {re.sub(r'fitm-c[0-9]+s[0-9]+', argv[2], m[1])}\n"
            lines.append(f"    --inherit-fd \"fd[{m[0]}]:{m[1][1:]}\" \\")

    lines.append("    && echo 'OK'")

    open("./restore.sh", "w").write("\n".join(lines).replace("## TEMPLATE ##", open_fds))

if __name__ == "__main__":
    main()