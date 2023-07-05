#include <stdio.h>
#include <stdlib.h>

int main(int argc, char *argv[]) {
  // Check if the correct number of command line arguments are provided
  if (argc != 3) {
    printf("Usage: %s <filename> <buffer_size>\n", argv[0]);
    return 1;
  }

  // Read command line arguments
  char *filename = argv[1];
  int buffer_size = atoi(argv[2]);

  // Open the file
  FILE *file = fopen(filename, "r");
  if (file == NULL) {
    printf("Error: File '%s' not found.\n", filename);
    return 1;
  }

  // Read and print the file contents
  char *buffer = (char *)malloc(buffer_size);
  while (fgets(buffer, buffer_size, file) != NULL) {
    printf("%s", buffer);
  }

  // Close the file and free the buffer memory
  fclose(file);

  return 0;
}
