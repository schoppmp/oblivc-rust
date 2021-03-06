
/// Arguments to the Obliv-C function [`millionnaire`][1].
///
/// After executing the protocol, `output` will be less than, equal to or
/// greater than zero if party 1's `input` was less than, equal to or greater
/// than party 2's.
///
/// [1]: fn.millionaire.html
typedef struct {
  int input;
  char output;
} millionaire_args;

/// The Obliv-C function that gets passed to [`oblivc::ProtocolDesc::exec_yao_protocol`][1].
///
/// `arg` should be of type [`millionaire_args`][2].
/// Automatically generated by [`oblivc::bindings`][3].
///
/// [1]: ../oblivc/struct.ProtocolDesc.html#method.exec_yao_protocol
/// [2]: struct.millionaire_args.html
/// [3]: ../oblivc/fn.bindings.html
void millionaire(void *arg);
