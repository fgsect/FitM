#include <fcntl.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

// ignore return value
int _ = 0;

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
    // pwd is set to the current state folder.
    FILE *f = fopen("envfile", "r");
    if (!f) {
        return getenv(var);
    }

    fseek(f, 0, SEEK_END);
    long fsize = ftell(f);
    fseek(f, 0, SEEK_SET);  /* same as rewind(f); */

    char *string = calloc(fsize + 1, 1);
    if (!string) {
        perror("alloc");
        exit(-2);
    }
    _ = fread(string, 1, fsize, f);
    fclose(f);

    char *tmp = string;

    while (!found && tmp) {
        if (!strncmp(var, tmp, strlen(var))) {
            found = tmp;
        }
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
    char *ret = (char *)calloc(strlen(found), 1);
    if (!ret) {
        perror("alloc");
        exit(-1);
    }
    strncpy(ret, found, strlen(found));
    free(string);
    return ret;
}