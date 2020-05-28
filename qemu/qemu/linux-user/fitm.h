#include <fcntl.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

char *getenv_from_file(const char *var);

/*
 * Checks if an `envfile` is present. If not checks getenv for the variable.
 * Note: env is not checked if the file is present.
 * Also need to handle NULL as return value in the caller
*/
char *getenv_from_file(const char *var) {
    char *found = NULL;

    // https://stackoverflow.com/questions/14002954/c-programming-how-to-read-the-whole-file-contents-into-a-buffer
    // Read the entire env file
    FILE *f = fopen("./envfile", "r");
    if (!f) {
        return getenv(var);
    }

    fseek(f, 0, SEEK_END);
    long fsize = ftell(f);
    fseek(f, 0, SEEK_SET);  /* same as rewind(f); */

    char *string = malloc(fsize + 1);
    fread(string, 1, fsize, f);
    fclose(f);

    char *tmp = string;

    while (!found && tmp) {
        if (!strncmp(var, tmp, strlen(var)))
            found = tmp;
        tmp = strchr(tmp, '\n');
        if (tmp) {
            *tmp = '\0';
            tmp++;
        }
    }

    if (!found) {
        free(string);
        return NULL;
    }

    found = strchr(found, '=');
    if (!found) {
        free(string);
        return NULL;
    }

    found++;
    char *ret = (char *)malloc(strlen(found));
    strncpy(ret, found, strlen(found));
    free(string);
    return ret;
}